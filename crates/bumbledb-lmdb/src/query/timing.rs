fn elapsed_micros(start: Instant) -> u128 {
    start.elapsed().as_micros()
}

fn finish_timings(timings: &mut QueryTimings, total_start: Instant) {
    timings.total_micros = elapsed_micros(total_start);
    timings.refresh_unaccounted();
}

fn allocation_delta_since(start: allocation::AllocationSnapshot) -> AllocationPhaseStats {
    allocation::delta(start, allocation::snapshot()).into()
}

