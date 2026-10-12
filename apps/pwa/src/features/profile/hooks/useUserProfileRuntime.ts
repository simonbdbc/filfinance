import type { AuthSessionClient } from "@shared/api/authSession";
import { useCallback, useEffect, useState } from "react";

import type { CurrentUserProfileResponse } from "@shared/api/contracts";
import { loadCurrentUserProfile } from "../services/userProfileApi";

type UseUserProfileRuntimeOptions = {
  authSession: AuthSessionClient | null;
  wsUrl: string;
};

export function useUserProfileRuntime({
  authSession,
  wsUrl,
}: UseUserProfileRuntimeOptions) {
  const [currentUserProfile, setCurrentUserProfile] =
    useState<CurrentUserProfileResponse | null>(null);
  const [userProfileRole, setUserProfileRole] = useState<string | null>(null);
  const [loadedUserProfileScope, setLoadedUserProfileScope] = useState<
    string | null
  >(null);
  const [userProfileError, setUserProfileError] = useState<string | null>(null);
  const currentUserProfileScope = `${wsUrl}\n${authSession?.id ?? ""}`;
  const resetUserProfileState = useCallback(() => {
    setCurrentUserProfile(null);
    setUserProfileRole(null);
    setLoadedUserProfileScope(null);
    setUserProfileError(null);
  }, []);
  const applyLoadedUserProfile = useCallback(
    (profile: CurrentUserProfileResponse) => {
      setCurrentUserProfile(profile);
      setUserProfileRole(profile.role);
      setLoadedUserProfileScope(currentUserProfileScope);
      setUserProfileError(null);
    },
    [currentUserProfileScope],
  );

  useEffect(() => {
    if (!authSession) {
      resetUserProfileState();
      return undefined;
    }

    let cancelled = false;
    const abortController = new AbortController();

    setCurrentUserProfile(null);
    setUserProfileRole(null);
    setLoadedUserProfileScope(null);

    void loadCurrentUserProfile({
      wsUrl,
      authSession,
      signal: abortController.signal,
    })
      .then((profile) => {
        if (!cancelled) {
          applyLoadedUserProfile(profile);
        }
      })
      .catch((error) => {
        if (cancelled || abortController.signal.aborted) {
          return;
        }

        setUserProfileError(
          error instanceof Error
            ? `Impossible de charger le profil utilisateur : ${error.message}`
            : "Impossible de charger le profil utilisateur",
        );
      });

    return () => {
      cancelled = true;
      abortController.abort();
    };
  }, [authSession, applyLoadedUserProfile, resetUserProfileState, wsUrl]);

  const isUserProfileCurrent =
    loadedUserProfileScope === currentUserProfileScope;

  return {
    currentUserProfile,
    userProfileRole,
    userProfileError,
    isUserProfileCurrent,
    visibleUserProfile: isUserProfileCurrent ? currentUserProfile : null,
    visibleUserProfileRole: isUserProfileCurrent ? userProfileRole : null,
    resetUserProfileState,
  };
}
