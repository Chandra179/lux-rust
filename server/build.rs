//! Build script: re-embeds the SQL migrations when they change.

fn main() {
    // `sqlx::migrate!` embeds server/migrations; rebuild when it changes.
    println!("cargo:rerun-if-changed=migrations");
}
