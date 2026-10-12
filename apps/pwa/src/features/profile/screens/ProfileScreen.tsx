import { appViewDisplayNameByView, useAppRuntime } from "@application";
import { PageHeader } from "@shared/ui";
import { ProfilePage } from "../components/ProfilePage";

export function ProfileScreen() {
  const { profile } = useAppRuntime();

  return (
    <>
      <PageHeader title={appViewDisplayNameByView.profile} />

      <ProfilePage
        profile={profile.visibleUserProfile}
        isProfileCurrent={profile.isUserProfileCurrent}
        profileError={profile.userProfileError}
      />
    </>
  );
}
