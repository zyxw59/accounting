CREATE TABLE resources (
  id BIGINT PRIMARY KEY NOT NULL,
  type TEXT NOT NULL,
  resource JSONB
);

CREATE TABLE string_parameters (
  id BIGINT NOT NULL REFERENCES resources (id),
  param_name TEXT NOT NULL,
  param_value TEXT NOT NULL,
  PRIMARY KEY (id, param_name, param_value)
);

CREATE TABLE reference_parameters (
  id BIGINT NOT NULL REFERENCES resources (id),
  param_name TEXT NOT NULL,
  param_value BIGINT NOT NULL REFERENCES resources (id),
  PRIMARY KEY (id, param_name, param_value)
);

CREATE TABLE integer_parameters (
  id BIGINT NOT NULL REFERENCES resources (id),
  param_name TEXT NOT NULL,
  param_value INTEGER NOT NULL,
  PRIMARY KEY (id, param_name, param_value)
);

CREATE TABLE amount_parameters (
  id BIGINT NOT NULL REFERENCES resources (id),
  param_name TEXT NOT NULL,
  param_value BIGINT NOT NULL,
  PRIMARY KEY (id, param_name, param_value)
);

CREATE TABLE date_parameters (
  id BIGINT NOT NULL REFERENCES resources (id),
  param_name TEXT NOT NULL,
  param_value DATE NOT NULL,
  PRIMARY KEY (id, param_name, param_value)
);

CREATE TABLE transaction_account_amount (
  id BIGINT NOT NULL REFERENCES resources (id),
  account BIGINT NOT NULL REFERENCES resources (id),
  amount BIGINT NOT NULL,
  PRIMARY KEY (id, account)
);

CREATE TABLE group_user_access (
  id BIGINT NOT NULL REFERENCES resources (id),
  user_ BIGINT NOT NULL REFERENCES resources (id),
  access BIGINT NOT NULL,
  PRIMARY KEY (id, user_)
);
