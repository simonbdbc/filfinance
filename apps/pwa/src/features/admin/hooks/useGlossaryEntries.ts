import type { AuthSessionClient } from "@shared/api/authSession";
import { useCallback, useEffect, useState } from "react";

import type {
  TranslationGlossaryCreatePayload,
  TranslationGlossaryEntry,
  TranslationGlossaryPreviewRequest,
  TranslationGlossaryPreviewResponse,
  TranslationGlossarySystemRule,
  TranslationGlossaryUpdatePayload,
} from "@shared/api/contracts";
import {
  createGlossaryEntry,
  disableGlossaryEntry,
  loadGlossaryEntries,
  loadGlossarySystemRules,
  previewGlossaryPipeline,
  reapplyGlossaryTranslations,
  updateGlossaryEntry,
} from "../services/glossaryApi";

type UseGlossaryEntriesOptions = {
  authSession: AuthSessionClient | null;
  isEnabled: boolean;
  wsUrl: string;
};

export function useGlossaryEntries({
  authSession,
  isEnabled,
  wsUrl,
}: UseGlossaryEntriesOptions) {
  const [entries, setEntries] = useState<TranslationGlossaryEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isPreviewing, setIsPreviewing] = useState(false);
  const [isReapplying, setIsReapplying] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [preview, setPreview] = useState<TranslationGlossaryPreviewResponse | null>(null);
  const [systemRules, setSystemRules] = useState<TranslationGlossarySystemRule[]>([]);

  const reload = useCallback(async () => {
    if (!isEnabled) {
      return;
    }

    setIsLoading(true);
    setError(null);
    try {
      const [entriesResponse, systemRulesResponse] = await Promise.all([
        loadGlossaryEntries({ authSession, wsUrl }),
        loadGlossarySystemRules({ authSession, wsUrl }),
      ]);
      setEntries(entriesResponse.items);
      setSystemRules(systemRulesResponse.items);
    } catch (nextError) {
      setError(
        nextError instanceof Error
          ? nextError.message
          : "Impossible de charger le glossaire.",
      );
    } finally {
      setIsLoading(false);
    }
  }, [authSession, isEnabled, wsUrl]);

  useEffect(() => {
    void reload();
  }, [reload]);

  const createEntry = useCallback(
    async (payload: TranslationGlossaryCreatePayload) => {
      setIsSaving(true);
      setError(null);
      try {
        const entry = await createGlossaryEntry({ authSession, payload, wsUrl });
        setEntries((current) => sortEntries([entry, ...current]));
        return entry;
      } catch (nextError) {
        setError(
          nextError instanceof Error
            ? nextError.message
            : "Impossible de créer l’entrée.",
        );
        return null;
      } finally {
        setIsSaving(false);
      }
    },
    [authSession, wsUrl],
  );

  const updateEntry = useCallback(
    async (id: number, payload: TranslationGlossaryUpdatePayload) => {
      setIsSaving(true);
      setError(null);
      try {
        const entry = await updateGlossaryEntry({
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
            : "Impossible de mettre à jour l’entrée.",
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
        const entry = await disableGlossaryEntry({ authSession, id, wsUrl });
        setEntries((current) =>
          sortEntries(current.map((item) => (item.id === id ? entry : item))),
        );
        return entry;
      } catch (nextError) {
        setError(
          nextError instanceof Error
            ? nextError.message
            : "Impossible de désactiver l’entrée.",
        );
        return null;
      } finally {
        setIsSaving(false);
      }
    },
    [authSession, wsUrl],
  );

  const reapplyTranslations = useCallback(async () => {
    setIsReapplying(true);
    setError(null);
    try {
      return await reapplyGlossaryTranslations({ authSession, wsUrl });
    } catch (nextError) {
      setError(
        nextError instanceof Error
          ? nextError.message
          : "Impossible de mettre à jour les traductions.",
      );
      return null;
    } finally {
      setIsReapplying(false);
    }
  }, [authSession, wsUrl]);

  const previewPipeline = useCallback(
    async (payload: TranslationGlossaryPreviewRequest) => {
      setIsPreviewing(true);
      setError(null);
      try {
        const response = await previewGlossaryPipeline({ authSession, payload, wsUrl });
        setPreview(response);
        return response;
      } catch (nextError) {
        setError(
          nextError instanceof Error
            ? nextError.message
            : "Impossible de générer l’aperçu.",
        );
        return null;
      } finally {
        setIsPreviewing(false);
      }
    },
    [authSession, wsUrl],
  );

  return {
    createEntry,
    disableEntry,
    entries,
    error,
    isLoading,
    isPreviewing,
    isReapplying,
    isSaving,
    preview,
    previewPipeline,
    reload,
    reapplyTranslations,
    systemRules,
    updateEntry,
  };
}

function sortEntries(entries: TranslationGlossaryEntry[]) {
  return [...entries].sort((left, right) => {
    if (right.priority !== left.priority) {
      return right.priority - left.priority;
    }

    return left.source_term.localeCompare(right.source_term);
  });
}
