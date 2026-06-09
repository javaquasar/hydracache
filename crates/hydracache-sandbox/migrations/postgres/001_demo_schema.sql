create table if not exists users (
    id bigint primary key,
    name text not null
);

create table if not exists products (
    id bigint primary key,
    name text not null,
    price_cents bigint not null
);

create table if not exists orders (
    id bigint primary key,
    user_id bigint not null,
    product_id bigint not null,
    quantity bigint not null
);
