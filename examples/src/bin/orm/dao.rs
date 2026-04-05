//! # DAO API — floz ORM
//!
//! Demonstrates the Active Record style DAO API:
//! Create, Read, Update (with dirty-tracking), and Delete.
//!
//! ```sh
//! DATABASE_URL=postgres://localhost:5432/floz1 cargo run -p examples --bin orm_dao
//! ```

use floz::Db;

floz::schema! {
    model User("users") {
        id:         integer("id").auto_increment().primary(),
        name:       varchar("name", 100),
        email:      varchar("email", 255).nullable().unique(),
        age:        short("age"),
        is_active:  bool("is_active").default("true"),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Db::connect_env("DATABASE_URL", "postgres://localhost:5432/floz1").await?;

    // Manage schema using generated DAO DDL methods
    User::drop_table(&db).await?;
    User::create_table(&db).await?;

    println!("--- DAO Active Record Example ---");

    // 1. Create (Builder)
    println!("> Creating user...");
    let alice = User {
        name: "Alice".to_string(),
        age: 30,
        email: Some("alice@example.com".to_string()),
        ..Default::default()
    }.create(&db).await?;
    println!("  Created: {:?}", alice);

    // 2. Read
    println!("> Reading user by ID...");
    let mut user = User::get(alice.id, &db).await?;
    println!("  Fetched: {:?}", user);

    // 3. Update (Dirty Tracking)
    // Only the changed fields will be included in the UPDATE statement!
    println!("> Updating user name and age (dirty tracking avoids updating email)...");
    user.set_name("Alice Updated".to_string());
    user.set_age(31);
    
    // Check our dirty changes before saving
    user.save(&db).await?;
    
    let updated = User::get(alice.id, &db).await?;
    println!("  Updated: {:?}", updated);

    // 4. Delete
    println!("> Deleting user...");
    updated.delete(&db).await?;
    
    // Verify deletion
    let deleted = User::find(alice.id, &db).await?;
    assert!(deleted.is_none());
    println!("  User successfully deleted.");

    Ok(())
}
