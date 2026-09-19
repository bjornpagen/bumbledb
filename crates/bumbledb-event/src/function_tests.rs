use crate::{
    ArithmeticLimits, BoolOp4, Capacity, CoordinateMap, DensityPiece, Error, ExactArithmetic,
    ExactRational, FiniteFunction, FiniteKernel, FunctionLimits, FunctionPiece, LawLimits, Limits,
    Space, SpaceId,
};

fn math() -> ExactArithmetic<'static> {
    ExactArithmetic::new(ArithmeticLimits::default(), &())
}
fn q(n: i64, d: i64) -> ExactRational {
    ExactRational::fraction(&n.to_string(), &d.to_string(), &mut math()).unwrap()
}
fn space(id: u8, bits: u8) -> Space {
    Space::new(SpaceId([id; 32]), bits, &()).unwrap()
}
fn function(space: &Space, values: &[i64], denominator: i64) -> FiniteFunction {
    let pieces: Vec<_> = values
        .iter()
        .enumerate()
        .filter(|(world, _)| space.full().contains(*world as u64).is_ok())
        .map(|(world, &value)| FunctionPiece {
            region: space
                .table((1 << space.dimensions()) - 1, &[1 << world], &())
                .unwrap(),
            value: q(value, denominator),
        })
        .collect();
    FiniteFunction::new(space, &pieces, FunctionLimits::default(), &mut math()).unwrap()
}
fn coin_prior() -> Space {
    let raw = space(70, 1);
    raw.with_density(
        &[DensityPiece {
            region: raw.full(),
            density: q(1, 2),
        }],
        LawLimits::default(),
        &mut math(),
    )
    .unwrap()
}
fn biased_channel(parent: &Space, id: u8) -> FiniteKernel {
    let extended = space(id, parent.dimensions() + 1);
    let coordinates: Vec<_> = (0..parent.dimensions()).collect();
    let map = CoordinateMap::coordinates(&extended, parent, &coordinates, &()).unwrap();
    let latent = extended.coordinate(0, &()).unwrap();
    let draw = extended.coordinate(parent.dimensions(), &()).unwrap();
    let matching = latent.apply(BoolOp4::EQUIVALENCE, &draw, &()).unwrap();
    let density = FiniteFunction::new(
        &extended,
        &[
            FunctionPiece {
                region: matching.clone(),
                value: q(3, 4),
            },
            FunctionPiece {
                region: matching.complement(),
                value: q(1, 4),
            },
        ],
        FunctionLimits::default(),
        &mut math(),
    )
    .unwrap();
    FiniteKernel::new(&map, &density, FunctionLimits::default(), &mut math()).unwrap()
}

#[test]
fn pointwise_function_algebra_preserves_signed_values_and_canonical_cells() {
    let base = space(1, 2);
    let left = function(&base, &[-2, 0, 1, 3], 2);
    let right = function(&base, &[4, -1, 0, 2], 3);
    assert_same_arena_images(&base, &left);
    let sum = left
        .add(&right, FunctionLimits::default(), &mut math())
        .unwrap();
    let product = left
        .multiply(&right, FunctionLimits::default(), &mut math())
        .unwrap();
    for (world, (a, b)) in [-2, 0, 1, 3].into_iter().zip([4, -1, 0, 2]).enumerate() {
        assert_eq!(sum.at(world as u64, &()).unwrap(), q(3 * a + 2 * b, 6));
        assert_eq!(product.at(world as u64, &()).unwrap(), q(a * b, 6));
    }
    let one = function(&base, &[1, 1, 1, 1], 1);
    assert_eq!(one.pieces().len(), 1);
    let merged =
        FiniteFunction::constant(&base, q(1, 1), FunctionLimits::default(), &mut math()).unwrap();
    assert!(one.equivalent(&merged, &()).unwrap());
    let independent = Space::with_order(base.identity(), &[1, 0], Limits::default(), &()).unwrap();
    assert!(
        left.align_to(&independent, FunctionLimits::default(), &mut math())
            .unwrap()
            .equivalent(&left, &())
            .unwrap()
    );
    let cancellation = left
        .multiply(
            &FiniteFunction::constant(&base, q(-1, 1), FunctionLimits::default(), &mut math())
                .unwrap(),
            FunctionLimits::default(),
            &mut math(),
        )
        .unwrap();
    assert!(
        left.add(&cancellation, FunctionLimits::default(), &mut math())
            .unwrap()
            .is_zero()
    );
}

#[test]
fn weighted_images_match_all_four_world_maps_and_nonrectangular_supports() {
    let target = space(3, 2);
    let mut cases = 0;
    for support in 1u64..16 {
        let base = space(2, 2);
        let source = base
            .restrict(&base.table(3, &[support], &()).unwrap(), &())
            .unwrap();
        let weights = [-2, 0, 3, 1];
        let input = function(&source, &weights, 1);
        for code in 0u64..256 {
            let mapped: Vec<_> = (0..4).map(|world| (code >> (2 * world)) & 3).collect();
            let readouts: Vec<_> = (0..2)
                .map(|bit| {
                    let truth = mapped.iter().enumerate().fold(0, |out, (world, value)| {
                        out | (((value >> bit) & 1) << world)
                    });
                    source.table(3, &[truth], &()).unwrap()
                })
                .collect();
            let map = CoordinateMap::new(&source, &target, &readouts, &()).unwrap();
            let pushed = input
                .pushforward(&map, FunctionLimits::default(), &mut math())
                .unwrap();
            for world in 0..4 {
                let expected: i64 = (0..4)
                    .filter(|&w| support & (1 << w) != 0 && mapped[w] == world)
                    .map(|w| weights[w])
                    .sum();
                assert_eq!(
                    pushed.at(world, &()).unwrap(),
                    ExactRational::from(expected)
                );
            }
            let output = function(&target, &[2, -1, 4, 0], 1);
            let pullback = output
                .pullback(&map, FunctionLimits::default(), &mut math())
                .unwrap();
            for world in 0..4 {
                if support & (1 << world) != 0 {
                    assert_eq!(
                        pullback.at(world, &()).unwrap(),
                        output
                            .at(mapped[usize::try_from(world).unwrap()], &())
                            .unwrap()
                    );
                }
            }
            cases += 1;
        }
    }
    assert_eq!(cases, 3840);
}

#[test]
fn weighted_images_sum_each_skipped_bit_once_and_keep_copied_readouts_correlated() {
    let source = space(4, 62);
    let input = FiniteFunction::constant(
        &source,
        q(1, 1i64 << 62),
        FunctionLimits::default(),
        &mut math(),
    )
    .unwrap();
    let target = Space::with_order(
        SpaceId([5; 32]),
        &(0..62).rev().collect::<Vec<_>>(),
        Limits::default(),
        &(),
    )
    .unwrap();
    let identity =
        CoordinateMap::coordinates(&source, &target, &(0..62).collect::<Vec<_>>(), &()).unwrap();
    let same = input
        .pushforward(&identity, FunctionLimits::default(), &mut math())
        .unwrap();
    assert_eq!(same.pieces().len(), 1);
    assert_eq!(same.at(0, &()).unwrap(), q(1, 1i64 << 62));
    let two = space(6, 2);
    let parity = source
        .coordinate(0, &())
        .unwrap()
        .apply(BoolOp4::XOR, &source.coordinate(61, &()).unwrap(), &())
        .unwrap();
    let map = CoordinateMap::new(&source, &two, &[parity.clone(), parity], &()).unwrap();
    let output = input
        .pushforward(&map, FunctionLimits::default(), &mut math())
        .unwrap();
    assert_eq!(output.at(0, &()).unwrap(), q(1, 2));
    assert_eq!(output.at(3, &()).unwrap(), q(1, 2));
    assert_eq!(output.at(1, &()).unwrap(), q(0, 1));
    assert_eq!(output.at(2, &()).unwrap(), q(0, 1));
    let measured = output.designate(LawLimits::default(), &mut math()).unwrap();
    assert_eq!(measured.full().count(&()).unwrap(), 4);
    let unit = space(7, 0);
    let discard = CoordinateMap::coordinates(&source, &unit, &[], &()).unwrap();
    assert_eq!(
        input
            .pushforward(&discard, FunctionLimits::default(), &mut math())
            .unwrap()
            .at(0, &())
            .unwrap(),
        ExactRational::one()
    );
}

#[test]
fn conditional_channels_preserve_priors_and_fresh_draws_share_latent_information() {
    let prior = coin_prior();
    let first = biased_channel(&prior, 71)
        .close(
            &prior,
            FunctionLimits::default(),
            LawLimits::default(),
            &mut math(),
        )
        .unwrap();
    let second = biased_channel(first.space(), 72)
        .close(
            first.space(),
            FunctionLimits::default(),
            LawLimits::default(),
            &mut math(),
        )
        .unwrap();
    let old = first.space().coordinate(1, &()).unwrap();
    let copied = second.parent().pullback(&old, &()).unwrap();
    let fresh = second.space().coordinate(2, &()).unwrap();
    assert_eq!(old.mass(&mut math()).unwrap(), q(1, 2));
    assert_eq!(copied.mass(&mut math()).unwrap(), q(1, 2));
    assert_eq!(fresh.mass(&mut math()).unwrap(), q(1, 2));
    assert_eq!(
        copied
            .apply(BoolOp4::AND, &fresh, &())
            .unwrap()
            .mass(&mut math())
            .unwrap(),
        q(5, 16)
    );
    assert_eq!(
        copied
            .apply(BoolOp4::AND, &copied, &())
            .unwrap()
            .mass(&mut math())
            .unwrap(),
        q(1, 2)
    );
    let disagreement = copied.apply(BoolOp4::XOR, &fresh, &()).unwrap();
    assert_eq!(disagreement.mass(&mut math()).unwrap(), q(3, 8));
    assert_eq!(
        copied
            .probability(&disagreement, &mut math())
            .unwrap()
            .value(&mut math())
            .unwrap(),
        Some(q(1, 2))
    );
    let marginal = FiniteFunction::density(second.space(), FunctionLimits::default(), &mut math())
        .unwrap()
        .pushforward(second.parent(), FunctionLimits::default(), &mut math())
        .unwrap();
    assert!(
        marginal
            .equivalent(
                &FiniteFunction::density(first.space(), FunctionLimits::default(), &mut math())
                    .unwrap(),
                &()
            )
            .unwrap()
    );
    let bytes = fresh.to_bytes(&()).unwrap();
    drop(first);
    drop(second);
    drop(prior);
    assert_eq!(
        crate::Event::from_bytes(&bytes, &())
            .unwrap()
            .mass(&mut math())
            .unwrap(),
        q(1, 2)
    );
}

#[test]
fn coup_tax_channel_updates_another_players_holding_through_the_shared_deck() {
    let raw = space(80, 2); // bits: Bob has Duke, Cleo has Duke
    let prior = function(&raw, &[36, 19, 19, 4], 78)
        .designate(LawLimits::default(), &mut math())
        .unwrap();
    let extended = space(81, 3); // bit 2: Bob declares Tax
    let parent = CoordinateMap::coordinates(&extended, &prior, &[0, 1], &()).unwrap();
    let matching = extended
        .coordinate(0, &())
        .unwrap()
        .apply(
            BoolOp4::EQUIVALENCE,
            &extended.coordinate(2, &()).unwrap(),
            &(),
        )
        .unwrap();
    let forecast = FiniteFunction::new(
        &extended,
        &[
            FunctionPiece {
                region: matching.clone(),
                value: q(4, 5),
            },
            FunctionPiece {
                region: matching.complement(),
                value: q(1, 5),
            },
        ],
        FunctionLimits::default(),
        &mut math(),
    )
    .unwrap();
    let kernel =
        FiniteKernel::new(&parent, &forecast, FunctionLimits::default(), &mut math()).unwrap();
    let closed = kernel
        .close(
            &prior,
            FunctionLimits::default(),
            LawLimits::default(),
            &mut math(),
        )
        .unwrap();
    let bob = closed
        .parent()
        .pullback(&prior.coordinate(0, &()).unwrap(), &())
        .unwrap();
    let cleo = closed
        .parent()
        .pullback(&prior.coordinate(1, &()).unwrap(), &())
        .unwrap();
    let tax = closed.space().coordinate(2, &()).unwrap();
    assert_eq!(
        bob.probability(&tax, &mut math())
            .unwrap()
            .value(&mut math())
            .unwrap(),
        Some(q(92, 147))
    );
    assert_eq!(
        cleo.probability(&tax, &mut math())
            .unwrap()
            .value(&mut math())
            .unwrap(),
        Some(q(5, 21))
    );
    assert_eq!(tax.mass(&mut math()).unwrap(), q(49, 130));
    let repeated = tax.apply(BoolOp4::AND, &tax, &()).unwrap();
    assert_eq!(
        cleo.probability(&repeated, &mut math())
            .unwrap()
            .value(&mut math())
            .unwrap(),
        Some(q(5, 21))
    );
    assert_actor_information(&extended, &parent, &kernel);
}

fn assert_actor_information(extended: &Space, parent: &CoordinateMap, kernel: &FiniteKernel) {
    let visible = space(82, 2); // Bob holding and proposed outcome, no Cleo holding
    let observation = CoordinateMap::coordinates(extended, &visible, &[0, 2], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    assert!(kernel.factors_through(&observation, &()).unwrap());
    let hidden = extended
        .coordinate(1, &())
        .unwrap()
        .apply(
            BoolOp4::EQUIVALENCE,
            &extended.coordinate(2, &()).unwrap(),
            &(),
        )
        .unwrap();
    let cheating = FiniteFunction::new(
        extended,
        &[
            FunctionPiece {
                region: hidden.clone(),
                value: q(4, 5),
            },
            FunctionPiece {
                region: hidden.complement(),
                value: q(1, 5),
            },
        ],
        FunctionLimits::default(),
        &mut math(),
    )
    .unwrap();
    assert!(
        !FiniteKernel::new(parent, &cheating, FunctionLimits::default(), &mut math())
            .unwrap()
            .factors_through(&observation, &())
            .unwrap()
    );
}

#[test]
fn normalization_checks_zero_prior_rows_and_rebinding_is_explicit() {
    let raw = space(90, 1);
    let prior = function(&raw, &[1, 0], 1)
        .designate(LawLimits::default(), &mut math())
        .unwrap();
    let extended = space(91, 2);
    let parent = CoordinateMap::coordinates(&extended, &prior, &[0], &()).unwrap();
    let broken = function(&extended, &[1, 0, 0, 0], 1);
    assert_eq!(
        FiniteKernel::new(&parent, &broken, FunctionLimits::default(), &mut math()).err(),
        Some(Error::KernelNotNormalized)
    );
    let negative = function(&extended, &[2, 2, -1, -1], 1);
    assert_eq!(
        FiniteKernel::new(&parent, &negative, FunctionLimits::default(), &mut math()).err(),
        Some(Error::NegativeMass)
    );
    let kernel = biased_channel(&prior, 92);
    let replacement = function(&raw, &[0, 1], 1)
        .designate(LawLimits::default(), &mut math())
        .unwrap();
    let closed = kernel
        .close(
            &replacement,
            FunctionLimits::default(),
            LawLimits::default(),
            &mut math(),
        )
        .unwrap();
    assert_eq!(
        closed
            .space()
            .coordinate(1, &())
            .unwrap()
            .mass(&mut math())
            .unwrap(),
        q(3, 4)
    );
    assert_eq!(
        kernel
            .close(
                &raw,
                FunctionLimits::default(),
                LawLimits::default(),
                &mut math()
            )
            .err(),
        Some(Error::MissingLaw)
    );
    assert_eq!(
        kernel
            .close(
                &coin_prior(),
                FunctionLimits::default(),
                LawLimits::default(),
                &mut math()
            )
            .err(),
        Some(Error::SpaceMismatch)
    );
}

#[test]
fn function_context_capacity_and_cancellation_faults_cannot_be_simplified_away() {
    struct Stop;
    impl crate::Control for Stop {
        fn checkpoint(&self) -> crate::Result<()> {
            Err(Error::Cancelled)
        }
    }
    let raw = space(93, 1);
    let zero =
        FiniteFunction::constant(&raw, q(0, 1), FunctionLimits::default(), &mut math()).unwrap();
    let other = FiniteFunction::constant(
        &space(94, 1),
        q(0, 1),
        FunctionLimits::default(),
        &mut math(),
    )
    .unwrap();
    assert_eq!(
        zero.multiply(&other, FunctionLimits::default(), &mut math())
            .err(),
        Some(Error::SpaceMismatch)
    );
    let one =
        FiniteFunction::constant(&raw, q(1, 1), FunctionLimits::default(), &mut math()).unwrap();
    let map = CoordinateMap::identity(&raw, &()).unwrap();
    let no_memo = FunctionLimits {
        memo_entries: 0,
        ..FunctionLimits::default()
    };
    assert_eq!(
        one.pushforward(&map, no_memo, &mut math()).err(),
        Some(Error::Capacity(Capacity::FunctionMemo))
    );
    let no_steps = FunctionLimits {
        steps: 0,
        ..FunctionLimits::default()
    };
    assert_eq!(
        one.pushforward(&map, no_steps, &mut math()).err(),
        Some(Error::Capacity(Capacity::FunctionSteps))
    );
    let no_cells = FunctionLimits {
        cells: 0,
        ..FunctionLimits::default()
    };
    assert_eq!(
        one.pushforward(&map, no_cells, &mut math()).err(),
        Some(Error::Capacity(Capacity::FunctionCells))
    );
    assert_eq!(
        zero.pushforward(
            &map,
            FunctionLimits::default(),
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &Stop)
        )
        .err(),
        Some(Error::Cancelled)
    );
    assert_eq!(
        FiniteFunction::new(
            &raw,
            &[
                FunctionPiece {
                    region: raw.full(),
                    value: q(1, 1)
                },
                FunctionPiece {
                    region: raw.empty().complement(),
                    value: q(0, 1)
                },
            ],
            FunctionLimits::default(),
            &mut math()
        )
        .err(),
        Some(Error::FunctionOverlap)
    );
}

#[test]
fn categorical_channels_exclude_unused_codes_and_weighted_maps_compose() {
    let prior = coin_prior();
    let raw = space(96, 3);
    let invalid = raw
        .coordinate(1, &())
        .unwrap()
        .apply(BoolOp4::AND, &raw.coordinate(2, &()).unwrap(), &())
        .unwrap();
    let extended = raw.restrict(&invalid.complement(), &()).unwrap();
    let parent = CoordinateMap::coordinates(&extended, &prior, &[0], &()).unwrap();
    let channel = function(&extended, &[3, 1, 2, 2, 1, 3, 0, 0], 6);
    let kernel =
        FiniteKernel::new(&parent, &channel, FunctionLimits::default(), &mut math()).unwrap();
    let closed = kernel
        .close(
            &prior,
            FunctionLimits::default(),
            LawLimits::default(),
            &mut math(),
        )
        .unwrap();
    assert_eq!(closed.space().full().count(&()).unwrap(), 6);
    assert_eq!(
        closed.space().full().contains(7),
        Err(Error::IllegalWorld(7))
    );
    let category_raw = space(97, 2);
    let category = category_raw
        .restrict(&category_raw.table(3, &[7], &()).unwrap(), &())
        .unwrap();
    let choice = CoordinateMap::coordinates(closed.space(), &category, &[1, 2], &()).unwrap();
    let joint =
        FiniteFunction::density(closed.space(), FunctionLimits::default(), &mut math()).unwrap();
    let marginal = joint
        .pushforward(&choice, FunctionLimits::default(), &mut math())
        .unwrap();
    for code in 0..3 {
        assert_eq!(marginal.at(code, &()).unwrap(), q(1, 3));
    }
    let unit = space(98, 0);
    let discard = CoordinateMap::coordinates(&category, &unit, &[], &()).unwrap();
    let composed = joint
        .pushforward(
            &choice.then(&discard, &()).unwrap(),
            FunctionLimits::default(),
            &mut math(),
        )
        .unwrap();
    let staged = marginal
        .pushforward(&discard, FunctionLimits::default(), &mut math())
        .unwrap();
    assert!(composed.equivalent(&staged, &()).unwrap());
    assert_eq!(staged.at(0, &()).unwrap(), ExactRational::one());
}

fn assert_same_arena_images(base: &Space, input: &FiniteFunction) {
    let identity = CoordinateMap::identity(base, &()).unwrap();
    assert!(
        input
            .pushforward(&identity, FunctionLimits::default(), &mut math())
            .unwrap()
            .equivalent(input, &())
            .unwrap()
    );
    let restricted = base
        .restrict(&base.table(3, &[7], &()).unwrap(), &())
        .unwrap();
    let left = base.coordinate(0, &()).unwrap();
    let right = base.coordinate(1, &()).unwrap();
    let map = CoordinateMap::new(
        base,
        &restricted,
        &[
            left.apply(BoolOp4::DIFFERENCE, &right, &()).unwrap(),
            right.apply(BoolOp4::DIFFERENCE, &left, &()).unwrap(),
        ],
        &(),
    )
    .unwrap();
    let pushed = input
        .pushforward(&map, FunctionLimits::default(), &mut math())
        .unwrap();
    assert_eq!(pushed.at(0, &()).unwrap(), q(1, 2));
    assert_eq!(pushed.at(1, &()).unwrap(), q(0, 1));
    assert_eq!(pushed.at(2, &()).unwrap(), q(1, 2));
    assert_eq!(pushed.at(3, &()), Err(Error::IllegalWorld(3)));
    let discard = CoordinateMap::coordinates(&restricted, &space(99, 0), &[], &()).unwrap();
    assert_eq!(
        pushed
            .pushforward(&discard, FunctionLimits::default(), &mut math())
            .unwrap()
            .at(0, &())
            .unwrap(),
        ExactRational::one()
    );
}
