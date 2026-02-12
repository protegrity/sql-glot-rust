use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sqlglot_rust::{generate, parse, transpile, Dialect};

fn bench_parse_simple(c: &mut Criterion) {
    c.bench_function("parse simple SELECT", |b| {
        b.iter(|| {
            parse(
                black_box("SELECT a, b, c FROM users WHERE id > 10"),
                Dialect::Ansi,
            )
            .unwrap()
        })
    });
}

fn bench_parse_complex(c: &mut Criterion) {
    let sql = "SELECT u.id, u.name, o.total \
               FROM users AS u \
               INNER JOIN orders AS o ON u.id = o.user_id \
               WHERE u.active = TRUE AND o.total > 100 \
               ORDER BY o.total DESC \
               LIMIT 50";
    c.bench_function("parse complex SELECT with JOIN", |b| {
        b.iter(|| parse(black_box(sql), Dialect::Ansi).unwrap())
    });
}

fn bench_parse_cte(c: &mut Criterion) {
    let sql = "WITH cte AS (SELECT id, name FROM users WHERE active = TRUE) \
               SELECT cte.id, cte.name FROM cte WHERE cte.id > 10";
    c.bench_function("parse CTE query", |b| {
        b.iter(|| parse(black_box(sql), Dialect::Ansi).unwrap())
    });
}

fn bench_roundtrip(c: &mut Criterion) {
    let sql = "SELECT a, b FROM t WHERE a > 1 AND b < 10 ORDER BY a";
    c.bench_function("roundtrip parse+generate", |b| {
        b.iter(|| {
            let ast = parse(black_box(sql), Dialect::Ansi).unwrap();
            generate(&ast, Dialect::Ansi)
        })
    });
}

fn bench_transpile(c: &mut Criterion) {
    let sql = "SELECT CAST(x AS INT), SUBSTR(name, 1, 3) FROM users WHERE active = TRUE";
    c.bench_function("transpile Ansi -> Postgres", |b| {
        b.iter(|| {
            transpile(black_box(sql), Dialect::Ansi, Dialect::Postgres).unwrap()
        })
    });
}

criterion_group!(
    benches,
    bench_parse_simple,
    bench_parse_complex,
    bench_parse_cte,
    bench_roundtrip,
    bench_transpile
);
criterion_main!(benches);
