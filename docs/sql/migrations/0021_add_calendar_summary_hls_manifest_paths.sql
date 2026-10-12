ALTER TABLE calendar_day_summaries
ADD COLUMN playlist_path TEXT;

ALTER TABLE calendar_day_summaries
ADD COLUMN immutable_playlist_url TEXT;

ALTER TABLE calendar_day_summaries
ADD COLUMN current_playlist_url TEXT;
