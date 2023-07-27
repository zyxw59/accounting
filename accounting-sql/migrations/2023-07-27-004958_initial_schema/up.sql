CREATE TABLE resources (
  id BIGINT PRIMARY KEY NOT NULL,
  type TEXT NOT NULL,
  resource JSONB NOT NULL
);

CREATE TABLE singular_parameters (
  id BIGINT PRIMARY KEY NOT NULL REFERENCES resources (id),
  group_ BIGINT NOT NULL REFERENCES resources (id),
  name TEXT,
  description TEXT,
  date_ DATE
);

CREATE TABLE references_resource (
  id BIGINT NOT NULL REFERENCES resources (id),
  reference_id BIGINT NOT NULL REFERENCES resources (id),
  PRIMARY KEY (id, reference_id)
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
