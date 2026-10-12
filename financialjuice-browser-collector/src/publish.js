export async function loadControl(config) {
  const target = asBackendTarget(config);
  const response = await fetchBackend(target.controlEndpoint, {
    method: "GET",
    headers: {
      authorization: `Bearer ${target.ingestToken}`,
    },
  });

  const text = await response.text();
  const payload = parseJsonResponse(text);
  if (!response.ok) {
    const detail = payload?.error ?? text;
    throw new Error(`backend GET ${target.controlEndpoint} failed with ${response.status}: ${detail}`);
  }

  return payload;
}

export async function loadControls(config) {
  return Promise.all(
    config.targets.map(async (target) => {
      try {
        return {
          target,
          control: await loadControl(target),
          error: null,
        };
      } catch (error) {
        return {
          target,
          control: null,
          error,
        };
      }
    }),
  );
}

export async function publishStatus(config, status, message = null) {
  const target = asBackendTarget(config);
  return postJson(target.statusEndpoint, target.ingestToken, {
    status,
    message,
  });
}

export async function publishStatusToTargets(config, status, message = null, targets = config.targets) {
  return Promise.all(
    targets.map(async (target) => {
      try {
        return {
          target,
          result: await publishStatus(target, status, message),
          error: null,
        };
      } catch (error) {
        return {
          target,
          result: null,
          error,
        };
      }
    }),
  );
}

export async function publishNewsBatch(config, events) {
  const target = asBackendTarget(config);
  if (events.length === 0) {
    return { accepted: 0, dropped: 0 };
  }

  return postJson(target.newsEndpoint, target.ingestToken, {
    events: events.slice(0, config.maxBatchSize),
  });
}

function asBackendTarget(config) {
  return {
    ...config,
    name: config.name ?? "default",
    maxBatchSize: config.maxBatchSize,
  };
}

async function postJson(url, token, body) {
  const response = await fetchBackend(url, {
    method: "POST",
    headers: {
      authorization: `Bearer ${token}`,
      "content-type": "application/json",
    },
    body: JSON.stringify(body),
  });

  const text = await response.text();
  const payload = parseJsonResponse(text);
  if (!response.ok) {
    const detail = payload?.error ?? text;
    throw new Error(`backend POST ${url} failed with ${response.status}: ${detail}`);
  }

  return payload;
}

async function fetchBackend(url, request) {
  try {
    return await fetch(url, request);
  } catch (error) {
    throw new Error(`backend ${request.method} ${url} failed: ${formatFetchError(error)}`);
  }
}

function formatFetchError(error) {
  const cause = error?.cause;
  const details = [error?.message ?? String(error)];

  if (cause?.code) {
    details.push(cause.code);
  }
  if (cause?.address || cause?.port) {
    details.push(`${cause.address ?? "unknown"}:${cause.port ?? "unknown"}`);
  }
  if (cause?.message && cause.message !== error?.message) {
    details.push(cause.message);
  }

  return details.join(" - ");
}

function parseJsonResponse(text) {
  if (!text.trim()) {
    return null;
  }
  return JSON.parse(text);
}
