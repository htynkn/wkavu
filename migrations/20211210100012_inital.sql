CREATE TABLE tv
(
    id     INTEGER PRIMARY KEY AUTOINCREMENT,
    tvdbid varchar(512),
    tvname varchar(512),
    name   varchar(512),
    url    varchar(2048)
);


CREATE TABLE tv_seed
(
    id    INTEGER PRIMARY KEY AUTOINCREMENT,
    tv_id INTEGER,
    ep    INTEGER,
    name  varchar(512),
    url   varchar(2048)
);
