import { appViewDisplayNameByView, useAppRuntime } from "@application";
import { useAuth } from "@features/auth";
import { NewsFeedCard } from "../components/NewsFeedCard";

export function FeedScreen() {
  const { authSession } = useAuth();
  const { history, role, server } = useAppRuntime();

  return (
    <NewsFeedCard
      authSession={authSession}
      title={appViewDisplayNameByView.feed}
      items={history.dailyFeedHistory}
      isLoading={history.isDailyHistoryLoading}
      currentLocalDayKey={history.currentLocalDayKey}
      previousLocalDayKey={history.previousLocalDayKey}
      isPreviousDayVisible={history.isPreviousFeedDayVisible}
      onPreviousDayVisibilityChange={history.setIsPreviousFeedDayVisible}
      newsSourceStatuses={history.newsSourceStatuses}
      role={role}
      wsUrl={server.wsUrl}
    />
  );
}
