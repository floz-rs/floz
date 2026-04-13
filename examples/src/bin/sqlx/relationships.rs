use floz::prelude::*;
use floz::Db;

#[model("authors")]
pub struct Author {
    #[col(auto, key)]
    pub id: i32,
    #[col(max = 100)]
    pub name: Varchar,
    #[rel(has_many(model = "Book", foreign_key = "author_id"))]
    pub books: Vec<Book>,
}

#[model("books")]
pub struct Book {
    #[col(auto, key)]
    pub id: i32,
    #[col(max = 200)]
    pub title: Varchar,
    pub author_id: i32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Db::connect_env("DATABASE_URL", "postgres://localhost:5432/floz1").await?;

    let _ = db
        .execute_raw("DROP TABLE IF EXISTS books CASCADE", vec![])
        .await;
    let _ = db
        .execute_raw("DROP TABLE IF EXISTS authors CASCADE", vec![])
        .await;
    db.execute_raw(
        "CREATE TABLE authors (
            id SERIAL PRIMARY KEY,
            name VARCHAR(100) NOT NULL
        )",
        vec![],
    )
    .await?;
    db.execute_raw(
        "CREATE TABLE books (
            id SERIAL PRIMARY KEY,
            title VARCHAR(200) NOT NULL,
            author_id INT NOT NULL REFERENCES authors(id) ON DELETE CASCADE
        )",
        vec![],
    )
    .await?;

    println!("> Seeding an author and their books...");
    let king = Author {
        name: "Stephen King".into(),
        ..Default::default()
    }
    .create(&db)
    .await?;

    Book {
        title: "The Shining".into(),
        author_id: king.id,
        ..Default::default()
    }
    .create(&db)
    .await?;
    Book {
        title: "IT".into(),
        author_id: king.id,
        ..Default::default()
    }
    .create(&db)
    .await?;
    Book {
        title: "Misery".into(),
        author_id: king.id,
        ..Default::default()
    }
    .create(&db)
    .await?;

    println!("> Lazy loading directly from instance...");
    let author_instance = Author::get(king.id, &db).await?;

    let fetched_books: Vec<Book> = author_instance.fetch_books(&db).await?;

    for b in fetched_books {
        println!("  - {}", b.title);
    }

    println!("> Eager Preloading associated books...");
    let mut all_authors = Author::all(&db).await?;

    Author::preload_books(&mut all_authors, &db).await?;

    println!("All authors with loaded books:");
    for author in &all_authors {
        let loaded_books = &author.books;
        println!("  - Author: {}", author.name);
        for b in loaded_books {
            println!("      - '{}'", b.title);
        }
    }

    Ok(())
}
