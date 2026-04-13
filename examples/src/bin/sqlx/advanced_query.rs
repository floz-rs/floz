use floz::prelude::*;
use floz::{Db, Executor, SelectQuery};

#[model("products")]
pub struct Product {
    #[col(auto, key)]
    pub id: i32,
    #[col(max = 100)]
    pub name: Varchar,
    pub price: f32,
    #[col(default = "true")]
    pub in_stock: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Db::connect_env("DATABASE_URL", "postgres://localhost:5432/floz1").await?;

    let _ = db
        .execute_raw("DROP TABLE IF EXISTS products CASCADE", vec![])
        .await;
    db.execute_raw(
        "CREATE TABLE products (
            id SERIAL PRIMARY KEY,
            name VARCHAR(100) NOT NULL,
            price REAL NOT NULL,
            in_stock BOOLEAN NOT NULL DEFAULT true
        )",
        vec![],
    )
    .await?;

    // --- Seed Data ---
    println!("> Seeding data...");
    Product {
        name: "Laptop".into(),
        price: 1200.0,
        in_stock: true,
        ..Default::default()
    }
    .create(&db)
    .await?;
    Product {
        name: "Mouse".into(),
        price: 25.0,
        in_stock: true,
        ..Default::default()
    }
    .create(&db)
    .await?;
    Product {
        name: "Monitor".into(),
        price: 300.0,
        in_stock: false,
        ..Default::default()
    }
    .create(&db)
    .await?;
    Product {
        name: "Keyboard".into(),
        price: 150.0,
        in_stock: true,
        ..Default::default()
    }
    .create(&db)
    .await?;

    // --- Select Query Builder ---
    println!("> Select query builder...");
    let query = SelectQuery::new(ProductTable::TABLE_NAME)
        .where_(
            ProductTable::price
                .gt(50.0)
                .and(ProductTable::in_stock.eq(true)),
        )
        .order_by(ProductTable::price.desc())
        .limit(2);

    // Check generated SQL safely
    let (sql, params) = query.to_sql();
    println!("Generated SQL: {}", sql);
    println!("Bound params: {:?}", params);

    // Run query
    let products: Vec<Product> = db.fetch_all(&sql, params).await?;
    println!("Found products (> $50, in stock):");
    for p in products {
        println!("  - {} (${})", p.name, p.price);
    }

    Ok(())
}
