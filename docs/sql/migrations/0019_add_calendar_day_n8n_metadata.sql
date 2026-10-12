ALTER TABLE calendar_day_runs
ADD COLUMN n8n_workflow_id TEXT;

ALTER TABLE calendar_day_runs
ADD COLUMN n8n_workflow_name TEXT;

ALTER TABLE calendar_day_runs
ADD COLUMN n8n_execution_id TEXT;
