ALTER TABLE ai_profiles
ADD COLUMN execution_defaults_json TEXT NOT NULL DEFAULT '{}'
CHECK (json_valid(execution_defaults_json));

ALTER TABLE runs
ADD COLUMN model_parameters_json TEXT NOT NULL DEFAULT '{}'
CHECK (json_valid(model_parameters_json));
