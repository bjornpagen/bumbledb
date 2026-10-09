//! A statement over closed relations holds for their rows.
//@ error: refuted by row `Archived` of `Status`
//@ line: 16

bumbledb::schema! {
    pub Review;

    closed relation Phase as PhaseId = { Draft, Live };
    closed relation Status as StatusId {
        phase: u64 as PhaseId,
    } = {
        Open     { phase: Live },
        Archived { phase: 7 },
    };

    Status(phase) <= Phase(id);
}
