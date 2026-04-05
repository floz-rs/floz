use floz::prelude::*;

schema! {
    model Note("notes") {
        id:         integer("id").auto_increment().primary(),
        content:    text("content"),
    }
}

/// Custom application state to demonstrate context extensions
struct MyCustomState {
    app_name: String,
}

/// GET /notes
#[route(
    get: "/notes",
    tag: "Notes",
    desc: "Fetch all notes",
    resps: [(200, "List of notes", Json<Vec<Note>>)],
)]
async fn list_notes(state: State) -> Resp {
    let app_state = state.ext::<MyCustomState>();
    info!("Handling request for app: {}", app_state.app_name);
    
    match Note::all(&state.db()).await {
        Ok(notes) => Resp::Ok().json(&notes),
        Err(e) => Resp::InternalServerError().body(e.to_string()),
    }
}

/// POST /notes
#[route(
    post: "/notes",
    tag: "Notes",
    desc: "Create a new note",
    resps: [(201, "Created note", Json<Note>)],
)]
async fn create_note(state: State) -> Resp {
    let note = Note {
        content: "Shared pool note!".to_string(),
        ..Default::default()
    }.create(&state.db()).await;

    match note {
        Ok(n) => Resp::Created().json(&n),
        Err(e) => Resp::InternalServerError().body(e.to_string()),
    }
}

#[floz::main]
async fn main() -> std::io::Result<()> {
    if std::env::var("DATABASE_URL").is_err() {
        std::env::set_var("DATABASE_URL", "postgres://localhost:5432/floz1");
    }

    App::new()
        .with(MyCustomState { app_name: "Floz Demo App".to_string() })
        .on_start(|ctx| async move {
            let db = ctx.db();
            Note::drop_table(&db).await.unwrap();
            Note::create_table(&db).await.unwrap();
            Note { content: "Initial seed note".to_string(), ..Default::default() }
                .create(&db).await.unwrap();
        })
        .run()
        .await
}
