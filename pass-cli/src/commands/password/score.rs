use anyhow::Result;

pub fn score(password: &str) -> Result<()> {
    let score = pass::password::score(password);

    println!(
        "- Score: {} ({})",
        score.numeric_score, score.password_score
    );

    for penalty in score.penalties {
        println!("- Penalty: {penalty}");
    }

    Ok(())
}
