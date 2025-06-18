pub fn debug_scores(
    time: Res<Time>,
    mut timer: Local<Timer>,
    simulations: Query<(&Simulation, &Score), With<Simulation>>,
) {
    // Initialiser le timer la première fois
    if timer.duration() == std::time::Duration::ZERO {
        *timer = Timer::from_seconds(2.0, TimerMode::Repeating);
    }

    timer.tick(time.delta());

    if timer.just_finished() {
        let mut scores: Vec<(usize, f32)> = simulations
            .iter()
            .map(|(sim, score)| (sim.id, score.get()))
            .collect();

        // Trier par score décroissant
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        info!("=== Scores des simulations ===");
        for (id, score) in scores.iter().take(5) {
            info!("Simulation {}: {:.1} points", id, score);
        }

        if scores.len() > 5 {
            info!("... et {} autres simulations", scores.len() - 5);
        }
    }
}