import type { AuthSessionClient } from "@shared/api/authSession";
import { useCallback, useEffect, useState } from "react";

import type {
  NewsBlacklistCreatePayload,
  NewsBlacklistEntry,
  NewsBlacklistHit,
  NewsBlacklistUpdatePayload,
} from "@shared/api/contracts";
import {
  createNewsBlacklistEntry,
  disableNewsBlacklistEntry,
  loadNewsBlacklistEntries,
  loadNewsBlacklistHits,
  reapplyNewsBlacklist,
  updateNewsBlacklistEntry,
} from "../services/newsBlacklistApi";

type UseNewsBlacklistOptions = {
  authSession: AuthSessionClient | null;
  isEnabled: boolean;
  wsUrl: string;
};

export function useNewsBlacklist({
  authSession,
  isEnabled,
  wsUrl,
}: UseNewsBlacklistOptions) {
  const [entries, setEntries] = useState<NewsBlacklistEntry[]>([]);
  const [hits, setHits] = useState<NewsBlacklistHit[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isReapplying, setIsReapplying] = useState(false);
  const [isSaving, setIsSaving] = useState(false);

  const reload = useCallback(async () => {
    if (!isEnabled) {
      return;
    }

    setIsLoading(true);
    setError(null);
    try {
      const [entriesResponse, hitsResponse] = await Promise.all([
        loadNewsBlacklistEntries({ authSession, wsUrl }),
        loadNewsBlacklistHits({ authSession, wsUrl }),
      ]);
      setEntries(sortEntries(entriesResponse.items));
      setHits(hitsResponse.items);
    } catch (nextError) {
      setError(
        nextError instanceof Error
          ? nextError.message
          : "Impossible de charger la liste de blocage.",
      );
    } finally {
      setIsLoading(false);
    }
  }, [authSession, isEnabled, wsUrl]);

  useEffect(() => {
    void reload();
  }, [reload]);

  const createEntry = useCallback(
    async (payload: NewsBlacklistCreatePayload) => {
      setIsSaving(true);
      setError(null);
      try {
        const entry = await createNewsBlacklistEntry({ authSession, payload, wsUrl });
        setEntries((current) => sortEntries([entry, ...current]));
        return entry;
      } catch (nextError) {
        setError(
          nextError instanceof Error
            ? nextError.message
            : "Impossible de créer l’entrée de la liste de blocage.",
        );
        return null;
      } finally {
        setIsSaving(false);
      }
    },
    [authSession, wsUrl],
  );

  const updateEntry = useCallback(
    async (id: number, payload: NewsBlacklistUpdatePayload) => {
      setIsSaving(true);
      setError(null);
      try {
        const entry = await updateNewsBlacklistEntry({
          authSession,
          id,
          payload,
          wsUrl,
        });
        setEntries((current) =>
          sortEntries(current.map((item) => (item.id === id ? entry : item))),
        );
        return entry;
      } catch (nextError) {
        setError(
          nextError instanceof Error
            ? nextError.message
            : "Impossible de mettre à jour l’entrée de la liste de blocage.",
        );
        return null;
      } finally {
        setIsSaving(false);
      }
    },
    [authSession, wsUrl],
  );

  const disableEntry = useCallback(
    async (id: number) => {
      setIsSaving(true);
      setError(null);
      try {
        const entry = await disableNewsBlacklistEntry({ authSession, id, wsUrl });
        setEntries((current) =>
          sortEntries(current.map((item) => (item.id === id ? entry : item))),
        );
        return entry;
      } catch (nextError) {
        setError(
          nextError instanceof Error
            ? nextError.message
            : "Impossible de désactiver l’entrée de la liste de blocage.",
        );
        return null;
      } finally {
        setIsSaving(false);
      }
    },
    [authSession, wsUrl],
  );

  const reapplyBlacklist = useCallback(async () => {
    if (!isEnabled) {
      return null;
    }

    setIsReapplying(true);
    setError(null);
    try {
      const report = await reapplyNewsBlacklist({ authSession, wsUrl });
      await reload();
      return report;
    } catch (nextError) {
      setError(
        nextError instanceof Error
          ? nextError.message
          : "Impossible de mettre à jour la liste de blocage.",
      );
      return null;
    } finally {
      setIsReapplying(false);
    }
  }, [authSession, isEnabled, reload, wsUrl]);

  return {
    createEntry,
    disableEntry,
    entries,
    error,
    hits,
    isLoading,
    isReapplying,
    isSaving,
    reapplyBlacklist,
    reload,
    updateEntry,
  };
}

function sortEntries(entries: NewsBlacklistEntry[]) {
  return [...entries].sort((left, right) => {
    if (left.enabled !== right.enabled) {
      return left.enabled ? -1 : 1;
    }

    if (right.term.length !== left.term.length) {
      return right.term.length - left.term.length;
    }

    return left.term.localeCompare(right.term);
  });
}
