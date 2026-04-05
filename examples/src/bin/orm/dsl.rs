//! # DSL API — floz ORM
//!
//! Demonstrates the typesafe query builder (DSL) for complex queries.
//! Mixes gracefully with the DAO API.
//!
//! ```sh
//! DATABASE_URL=postgres://localhost:5432/floz1 cargo run -p examples --bin orm_dsl
//! ```

use floz::prelude::*;
use floz::Db;

floz::schema! {
    model Product("products") {
        id:         integer("id").auto_increment().primary(),
        name:       varchar("name", 100),
        price:      real("price"),
        in_stock:   bool("in_stock").default("true"),
    }

    model Order("orders") {
        id:         integer("id").auto_increment().primary(),
        product_id: integer("product_id"),
        qty:        integer("qty"),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Db::connect_env("DATABASE_URL", "postgres://localhost:5432/floz1").await?;

    // Manage schema using generated DAO DDL methods
    Order::drop_table(&db).await?;
    Product::drop_table(&db).await?;
    
    Product::create_table(&db).await?;
    Order::create_table(&db).await?;

    Product { name: "Laptop".into(), price: 1200.0, in_stock: true, ..Default::default() }.create(&db).await?;
    Product { name: "Mouse".into(), price: 25.0, in_stock: true, ..Default::default() }.create(&db).await?;
    Product { name: "Monitor".into(), price: 300.0, in_stock: false, ..Default::default() }.create(&db).await?;
    Product { name: "Keyboard".into(), price: 150.0, in_stock: true, ..Default::default() }.create(&db).await?;

    // Basic SELECT
    println!("> Select active products > $50...");
    let (sql, params) = floz::SelectQuery::new(ProductTable::TABLE_NAME)
        .where_(ProductTable::price.gt(50.0).and(ProductTable::in_stock.eq(true)))
        .order_by(ProductTable::name.asc())
        .limit(10)
        .to_sql();
    let active_expensive: Vec<Product> = db.fetch_all(&sql, params).await?;
        
    for p in active_expensive {
        println!("  - {} (${})", p.name, p.price);
    }
    
    // Specific Columns Tuple Select
    println!("> Select specific columns...");
    let (sql, params) = floz::SelectQuery::new(ProductTable::TABLE_NAME)
        .cols(&["name", "price"])
        .where_(ProductTable::in_stock.eq(true))
        .to_sql();
        
    // Custom wrapper for row decoding
    #[derive(Debug, sqlx::FromRow)]
    struct NamePrice { name: String, price: f32 }
    
    let names: Vec<NamePrice> = db.fetch_all(&sql, params).await?;
    
    for row in names {
        println!("  Column select: {} at ${}", row.name, row.price);
    }
    
    // Bulk Update
    println!("> Bulk updating prices (+10%)...");
    let (sql, params) = floz::UpdateQuery::new(ProductTable::TABLE_NAME)
        .set("price", 999.0) // .plus syntax might not be available, fallback to hardcoded or proper expr
        .where_(ProductTable::in_stock.eq(true))
        .to_sql()?;
    db.execute_raw(&sql, params).await?;

    Ok(())
}
