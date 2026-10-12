export class SessionExpiredError extends Error {
  constructor() {
    super("AUTH_SESSION_EXPIRED");
  }
}
export type TokenGrant = { token: string; expiresAt: number; subject: string };
let sessionSequence = 0;

/** One identity per login, independent of rotating bearer credentials. */
export class AuthSessionClient {
  readonly id = `session-${++sessionSequence}`;
  private token: string | null = null;
  private expiresAt = 0;
  private pending: Promise<string> | null = null;
  private disposed = false;
  private timer: ReturnType<typeof setTimeout> | undefined;
  readonly lifetime = new AbortController();
  readonly cache = new Map<
    string,
    { expiresAt: number; promise: Promise<unknown> }
  >();
  constructor(
    readonly userId: string,
    private readonly renew: () => Promise<TokenGrant>,
    private readonly notify: (
      state: "ready" | "unavailable" | "expired",
    ) => void = () => {},
  ) {}
  getToken() {
    return this.token;
  }
  get isActive() {
    return !this.disposed;
  }
  assertActive() {
    if (this.disposed) throw new SessionExpiredError();
  }
  close() {
    this.disposed = true;
    this.token = null;
    clearTimeout(this.timer);
    this.cache.clear();
    this.lifetime.abort();
  }
  async validToken() {
    this.assertActive();
    return this.token && this.expiresAt > Date.now()
      ? this.token
      : this.refresh();
  }
  /** Late failures from the previous token reuse a renewal already completed. */
  async recover(rejectedToken: string | null) {
    this.assertActive();
    if (
      this.token &&
      this.token !== rejectedToken &&
      this.expiresAt > Date.now()
    )
      return this.token;
    return this.refresh();
  }
  refresh(): Promise<string> {
    this.assertActive();
    if (this.pending) return this.pending;
    const request = Promise.resolve()
      .then(this.renew)
      .then((grant) => {
        this.assertActive();
        if (grant.subject !== this.userId || grant.expiresAt <= Date.now())
          throw new Error("Invalid token grant");
        this.token = grant.token;
        this.expiresAt = grant.expiresAt;
        clearTimeout(this.timer);
        this.timer = setTimeout(
          () => {
            void this.refresh().catch(() => {});
          },
          Math.max(1_000, this.expiresAt - Date.now() - 60_000),
        );
        this.notify("ready");
        return grant.token;
      })
      .catch((error) => {
        if (!this.disposed) {
          if (error instanceof SessionExpiredError) {
            this.close();
            this.notify("expired");
          } else {
            this.notify("unavailable");
            clearTimeout(this.timer);
            this.timer = setTimeout(() => {
              void this.refresh().catch(() => {});
            }, 30_000);
          }
        }
        throw error;
      })
      .finally(() => {
        if (this.pending === request) this.pending = null;
      });
    this.pending = request;
    return request;
  }
}

/** Scoped transport: call only for Fil Finance service requests, never third-party assets. */
export async function authenticatedFetch(
  session: AuthSessionClient | null,
  input: RequestInfo | URL,
  init: RequestInit = {},
): Promise<Response> {
  if (!session) throw new SessionExpiredError();
  session.assertActive();
  // Clone before sending, including bodies of Request inputs.
  const template = new Request(input, init);
  const controller = new AbortController();
  const abort = () => controller.abort();
  const originalSignal = template.signal;
  originalSignal.addEventListener("abort", abort, { once: true });
  session.lifetime.signal.addEventListener("abort", abort, { once: true });
  if (originalSignal.aborted || session.lifetime.signal.aborted) abort();
  const check = () => {
    session.assertActive();
    if (controller.signal.aborted)
      throw new DOMException("Request aborted", "AbortError");
  };
  const send = async (token: string) => {
    check();
    const headers = new Headers(template.headers);
    headers.set("Authorization", `Bearer ${token}`);
    const response = await fetch(
      new Request(template.clone(), { headers, signal: controller.signal }),
    );
    check();
    return response;
  };
  try {
    check();
    const token = await waitForToken(session.validToken(), controller.signal);
    let response = await send(token);
    if (response.status !== 401) return response;
    await response.body?.cancel();
    check();
    const renewed = await waitForToken(
      session.recover(token),
      controller.signal,
    );
    check();
    response = await send(renewed);
    return response;
  } finally {
    originalSignal.removeEventListener("abort", abort);
    session.lifetime.signal.removeEventListener("abort", abort);
  }
}

/** Cancel one waiting request without cancelling the shared token renewal. */
function waitForToken(
  promise: Promise<string>,
  signal: AbortSignal,
): Promise<string> {
  return new Promise((resolve, reject) => {
    const abort = () =>
      reject(new DOMException("Request aborted", "AbortError"));
    signal.addEventListener("abort", abort, { once: true });
    promise
      .then(resolve, reject)
      .finally(() => signal.removeEventListener("abort", abort));
    if (signal.aborted) abort();
  });
}
