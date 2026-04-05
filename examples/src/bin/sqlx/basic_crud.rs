use floz::prelude::*;
use floz::Db;

floz::schema! {
    model User("users") {
        id:         integer("id").auto_increment().primary(),
        name:       varchar("name", 100),
        age:        short("age"),
        is_active:  bool("is_active").default("true"),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Note: requires a running Postgres DB
    println!("Connecting to floz1...");
    let db = Db::connect_env("DATABASE_URL", "postgres://localhost:5432/floz1").await?;

    // Create table (warning: usually managed by migrations!)
    let _ = db.execute_raw("DROP TABLE IF EXISTS users CASCADE", vec![]).await;
    db.execute_raw(
        "CREATE TABLE users (
            id SERIAL PRIMARY KEY,
            name VARCHAR(100) NOT NULL,
            age SMALLINT NOT NULL,
            is_active BOOLEAN NOT NULL DEFAULT true
        )",
        vec![]
    ).await?;

    // --- 1. Create ---
    println!("> Creating a user...");
    let alice = User {
        name: "Alice".to_string(),
        age: 25,
        ..User::default()
    }.create(&db).await?;
    println!("Created: {:?}", alice);

    // --- 2. Read ---
    println!("> Reading user by ID...");
    let mut fetched = User::get(alice.id, &db).await?;
    println!("Fetched: {:?}", fetched);

    // --- 3. Update ---
    println!("> Updating user...");
    fetched.set_name("Alice (Updated)".to_string());
    fetched.set_age(26);
    fetched.save(&db).await?; // Only updates modified fields!
    
    let updated = User::get(alice.id, &db).await?;
    println!("Updated: {:?}", updated);

    // --- 4. Delete ---
    println!("> Deleting user...");
    updated.delete(&db).await?;
    
    let deleted = User::find(alice.id, &db).await?;
    assert!(deleted.is_none());
    println!("User successfully deleted.");

    Ok(())
}
