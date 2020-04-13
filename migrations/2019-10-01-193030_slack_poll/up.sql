CREATE TABLE poll
(
    id        SERIAL PRIMARY KEY,
    channel   TEXT NOT NULL,
    is_closed bool not null default false,
    time      text
);

CREATE TABLE poll_variant
(
    id         SERIAL PRIMARY KEY,
    day_id     integer   not null,
    title      text,
    variant    text,
    start_date timestamp not null,
    end_date   timestamp
);

create table channel_users
(
    id             SERIAL PRIMARY KEY,
    user_slack_id  TEXT,
    user_thumbnail TEXT
);

create unique index slack_id on channel_users (user_slack_id);

create table votes_results
(
    id                SERIAL PRIMARY KEY,
    user_id           integer NOT NULL,
    day_id            integer NOT NULL,
    poll_variant_id   integer NOT NULL,
    dialog_variant_id integer NOT NULL,
    score             integer NOT NULL
);

create table dialog_variants
(
    id           SERIAL PRIMARY KEY,
    day_id       integer not null,
    variant_text text    not null,
    max_score    integer not null
)
