insert into users (id, name) values
    (42, 'Ada'),
    (7, 'Linus')
on conflict(id) do update set
    name = excluded.name;

insert into products (id, name, price_cents) values
    (100, 'Mechanical Keyboard', 12900),
    (200, 'Observability Notebook', 1900)
on conflict(id) do update set
    name = excluded.name,
    price_cents = excluded.price_cents;

insert into orders (id, user_id, product_id, quantity) values
    (5000, 42, 100, 1),
    (5001, 7, 200, 2)
on conflict(id) do update set
    user_id = excluded.user_id,
    product_id = excluded.product_id,
    quantity = excluded.quantity;
