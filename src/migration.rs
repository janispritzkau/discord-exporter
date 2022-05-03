use eyre::Context;

const MIGRATIONS: &[&str] = &[include_str!("migrations/01_initial_schema.sql")];

pub fn run_migrations(conn: &mut rusqlite::Connection) -> eyre::Result<()> {
    let current_version: usize = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let latest_version = MIGRATIONS.len();

    if current_version > latest_version {
        eyre::bail!("cannot downgrade database")
    }

    let tx = conn.transaction()?;

    for (i, &sql) in MIGRATIONS.iter().enumerate() {
        if current_version <= i {
            tx.execute_batch(sql)
                .wrap_err("could not upgrade database to version {}")?;
        }
    }

    tx.pragma_update(None, "user_version", latest_version)?;
    tx.commit()?;

    Ok(())
}
