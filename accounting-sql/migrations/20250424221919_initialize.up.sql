CREATE TABLE transactions (
  id BIGINT PRIMARY KEY NOT NULL,
  description TEXT,
  date_ DATE
);

CREATE TABLE accounts (
  id BIGINT PRIMARY KEY NOT NULL,
  description TEXT,
  balance BIGINT NOT NULL
);

CREATE TABLE splits (
  transaction BIGINT NOT NULL REFERENCES transactions(id),
  account BIGINT NOT NULL REFERENCES accounts(id),
  note TEXT,
  amount BIGINT NOT NULL
);
