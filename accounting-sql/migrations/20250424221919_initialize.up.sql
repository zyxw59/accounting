CREATE TABLE transactions (
  id BIGINT PRIMARY KEY NOT NULL,
  description TEXT NOT NULL,
  date_ DATE NOT NULL
);

CREATE TABLE accounts (
  id BIGINT PRIMARY KEY NOT NULL,
  name TEXT NOT NULL,
  description TEXT,
  balance NUMERIC NOT NULL
);

CREATE TABLE splits (
  transaction BIGINT NOT NULL REFERENCES transactions(id),
  account BIGINT NOT NULL REFERENCES accounts(id),
  note TEXT,
  amount NUMERIC NOT NULL
);
