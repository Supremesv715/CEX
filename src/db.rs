use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

pub type DbPool = PgPool;



pub async fn init_db(database_url: &str) -> Result<DbPool, sqlx::Error> {
    

    

    let max_retries = 5;
    for attempt in 1..=max_retries {
        println!("DB connect attempt {}/{}", attempt, max_retries);

        let connect_future = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url);

        match tokio::time::timeout(Duration::from_secs(60), connect_future).await {
            Ok(Ok(pool)) => {
                println!("Connected to DB on attempt {}", attempt);
                

                sqlx::migrate!("./migrations").run(&pool).await?;
                return Ok(pool);
            }
            Ok(Err(e)) => {
                eprintln!("DB connect attempt {} failed: {}", attempt, e);
                if attempt == max_retries {
                    return Err(e);
                }
            }
            Err(_) => {
                eprintln!("DB connect attempt {} timed out after 60s", attempt);
                if attempt == max_retries {
                    return Err(sqlx::Error::PoolTimedOut);
                }
            }
        }

        

        let backoff_secs = 2u64.pow((attempt - 1) as u32);
        let backoff = Duration::from_secs(backoff_secs);
        println!("Waiting {:?} before retrying...", backoff);
        tokio::time::sleep(backoff).await;
    }

    

    Err(sqlx::Error::PoolTimedOut)
}
