use std::cell::Cell;

use crate::*;

fn world(id: u8, bits: u8, support: u64) -> Space {
    let raw = Space::new(SpaceId([id; 32]), bits, &()).unwrap();
    raw.restrict(&raw.table((1 << bits) - 1, &[support], &()).unwrap(), &())
        .unwrap()
}

fn root(space: &Space, mask: u64) -> Event {
    space
        .table((1 << space.dimensions()) - 1, &[mask], &())
        .unwrap()
}

fn erase(value: &AdmittedDescriptor) -> Descriptor {
    Descriptor::capture(value, DescriptorLimits::default(), &()).unwrap()
}

fn imported(value: &AdmittedDescriptor) -> AdmittedDescriptor {
    let descriptor = erase(value);
    let bytes = descriptor
        .to_bytes(DescriptorLimits::default(), &())
        .unwrap();
    let parsed = Descriptor::from_bytes(&bytes, DescriptorLimits::default(), &()).unwrap();
    assert_eq!(descriptor, parsed);
    assert_eq!(
        bytes,
        parsed.to_bytes(DescriptorLimits::default(), &()).unwrap()
    );
    let result = Descriptor::import(&bytes, DescriptorLimits::default(), &()).unwrap();
    assert_eq!(descriptor, erase(&result));
    result
}

fn base(space: &Space, environment: &Space) -> SurjectiveMap {
    CoordinateMap::new(space, environment, &[], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap()
}

#[test]
fn descriptors_reconstruct_asymmetric_supports_and_reversed_roles() {
    let environment = world(1, 0, 1);
    let a = world(2, 2, 7);
    let b = world(3, 1, 3);
    let product = FibreProduct::new(
        SpaceId([4; 32]),
        &base(&a, &environment),
        &base(&b, &environment),
        &(),
    )
    .unwrap();
    for reversed in [false, true] {
        let product = if reversed {
            product.converse()
        } else {
            product.clone()
        };
        for bits in 0..256 {
            if bits & !0b0111_0111 != 0 {
                continue;
            }
            let relation = WorldRelation::new(&product, &root(product.space(), bits), &()).unwrap();
            let AdmittedDescriptor::Relation(restored) =
                imported(&AdmittedDescriptor::Relation(relation.clone()))
            else {
                panic!("relation");
            };
            assert_eq!(
                relation.region().to_bytes(&()).unwrap(),
                restored.region().to_bytes(&()).unwrap()
            );
            assert_eq!(relation.input().identity(), restored.input().identity());
            assert_eq!(relation.output().identity(), restored.output().identity());
            for goal in 0..(1 << (1 << relation.output().dimensions())) {
                let goal = root(relation.output(), goal);
                for (expected, actual) in [
                    (
                        relation.may(&goal, &()).unwrap(),
                        restored.may(&goal, &()).unwrap(),
                    ),
                    (
                        relation.must(&goal, &()).unwrap(),
                        restored.must(&goal, &()).unwrap(),
                    ),
                ] {
                    assert_eq!(
                        expected.to_bytes(&()).unwrap(),
                        actual.to_bytes(&()).unwrap()
                    );
                }
            }
        }
    }
}

#[test]
fn every_descriptor_kind_roundtrips_and_survives_original_owner_drop() {
    let environment = world(5, 0, 1);
    let state = world(6, 1, 3);
    let base = base(&state, &environment);
    let product = FibreProduct::new(SpaceId([7; 32]), &base, &base, &()).unwrap();
    let map = CoordinateMap::new(
        &state,
        &state,
        &[state.coordinate(0, &()).unwrap().complement()],
        &(),
    )
    .unwrap();
    let faces = FaceProduct::new(
        SpaceId([8; 32]),
        &[base.clone(), base.clone(), base.clone()],
        &(),
    )
    .unwrap();
    let relation = WorldRelation::graph(&product, &map, &()).unwrap();
    let plan = RelationalProduct::new(SpaceId([9; 32]), &product, &product, &product, &()).unwrap();
    let square = product
        .certify_square(product.left().map(), product.right().map(), &())
        .unwrap();
    let originals = [
        AdmittedDescriptor::Map(map.clone()),
        AdmittedDescriptor::Surjective(map.certify_surjective(&()).unwrap()),
        AdmittedDescriptor::Faces(faces),
        AdmittedDescriptor::Fibre(product),
        AdmittedDescriptor::Relation(relation),
        AdmittedDescriptor::Composition(plan),
        AdmittedDescriptor::Square(square),
    ];
    let copies: Vec<_> = originals.iter().map(imported).collect();
    drop(originals);
    drop(base);
    drop(state);
    drop(environment);
    drop(map);
    let AdmittedDescriptor::Composition(plan) = &copies[5] else {
        panic!("plan");
    };
    let AdmittedDescriptor::Relation(relation) = &copies[4] else {
        panic!("relation");
    };
    let square = plan.compose(relation, relation, &()).unwrap();
    let identity = WorldRelation::identity(square.product(), &()).unwrap();
    assert!(square.equivalent(&identity, &()).unwrap());
}

#[test]
fn nonlinear_environment_and_physical_order_do_not_change_descriptors() {
    let source = world(10, 2, 15);
    let target = world(11, 1, 3);
    let parity = source
        .coordinate(0, &())
        .unwrap()
        .apply(BoolOp4::XOR, &source.coordinate(1, &()).unwrap(), &())
        .unwrap();
    let map = CoordinateMap::new(&source, &target, &[parity], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let forward = FibreProduct::with_order(
        SpaceId([12; 32]),
        &map,
        &map,
        &[0, 1, 2, 3],
        Limits::default(),
        &(),
    )
    .unwrap();
    let backward = FibreProduct::with_order(
        SpaceId([12; 32]),
        &map,
        &map,
        &[3, 2, 1, 0],
        Limits::default(),
        &(),
    )
    .unwrap();
    assert_eq!(
        erase(&AdmittedDescriptor::Fibre(forward.clone())),
        erase(&AdmittedDescriptor::Fibre(backward))
    );
    let AdmittedDescriptor::Fibre(restored) = imported(&AdmittedDescriptor::Fibre(forward)) else {
        panic!("fibre");
    };
    assert_eq!(restored.space().full().count(&()).unwrap(), 8);
    for encoded in 0..16 {
        assert_eq!(
            restored.space().full().contains(encoded).unwrap_or(false),
            ((encoded & 1) ^ ((encoded >> 1) & 1)) == (((encoded >> 2) & 1) ^ ((encoded >> 3) & 1))
        );
    }
}

#[test]
fn hostile_data_cannot_forge_full_markers_onto_maps_or_complete_squares() {
    let source = world(13, 1, 3);
    let pair = world(14, 2, 15);
    let copy = CoordinateMap::coordinates(&source, &pair, &[0, 0], &()).unwrap();
    let Descriptor::Map(map) = erase(&AdmittedDescriptor::Map(copy)) else {
        panic!("map");
    };
    let limits = DescriptorLimits::default();
    assert!(matches!(
        Descriptor::Surjective(map.clone()).admit(limits, &()),
        Err(Error::IncompleteImage)
    ));
    let mut proper = map.clone();
    proper.source = source.coordinate(0, &()).unwrap().to_bytes(&()).unwrap();
    assert!(matches!(
        Descriptor::Map(proper).admit(limits, &()),
        Err(Error::InvalidEncoding)
    ));
    let mut foreign = map;
    foreign.readouts[0] = pair.empty().to_bytes(&()).unwrap();
    assert!(matches!(
        Descriptor::Map(foreign).admit(limits, &()),
        Err(Error::SpaceMismatch)
    ));
    let environment = world(15, 0, 1);
    let endpoint = base(&source, &environment);
    let product = FibreProduct::new(SpaceId([16; 32]), &endpoint, &endpoint, &()).unwrap();
    let square = product
        .certify_square(product.left().map(), product.right().map(), &())
        .unwrap();
    let Descriptor::Square { product, left, .. } = erase(&AdmittedDescriptor::Square(square))
    else {
        panic!("square");
    };
    let forged = Descriptor::Square {
        product,
        right: left.clone(),
        left,
    };
    // Both individual projections are onto; their duplicate joint readout is not.
    assert!(matches!(
        forged.admit(limits, &()),
        Err(Error::IncompleteImage)
    ));
}

#[test]
fn changed_endpoint_and_environment_definitions_refuse_composition_import() {
    let environment = world(17, 1, 3);
    let states = world(18, 1, 3);
    let map = CoordinateMap::coordinates(&states, &environment, &[0], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let product = FibreProduct::new(SpaceId([19; 32]), &map, &map, &()).unwrap();
    let plan =
        RelationalProduct::new(SpaceId([20; 32]), &product, &product, &product, &()).unwrap();
    let descriptor = erase(&AdmittedDescriptor::Composition(plan));
    let Descriptor::Composition {
        identity,
        st,
        mut tu,
        su,
    } = descriptor
    else {
        panic!("plan");
    };
    let inverse = states
        .coordinate(0, &())
        .unwrap()
        .complement()
        .to_bytes(&())
        .unwrap();
    tu.left.readouts = vec![inverse];
    let incompatible = Descriptor::Composition {
        identity,
        st,
        tu,
        su,
    };
    assert!(matches!(
        incompatible.admit(DescriptorLimits::default(), &()),
        Err(Error::EnvironmentMismatch)
    ));
}

struct Stop(Cell<usize>);
impl Control for Stop {
    fn checkpoint(&self) -> Result<()> {
        let remaining = self.0.get();
        if remaining == 0 {
            return Err(Error::Cancelled);
        }
        self.0.set(remaining - 1);
        Ok(())
    }
}

#[test]
fn malformed_wire_limits_and_cancellation_never_publish_partial_imports() {
    let a = world(21, 1, 3);
    let map = CoordinateMap::coordinates(&a, &a, &[0], &()).unwrap();
    let descriptor = erase(&AdmittedDescriptor::Map(map));
    let limits = DescriptorLimits::default();
    let bytes = descriptor.to_bytes(limits, &()).unwrap();
    assert_eq!(&bytes[..6], b"BEDC\x01\x00");
    for end in 0..bytes.len() {
        assert!(Descriptor::from_bytes(&bytes[..end], limits, &()).is_err());
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(matches!(
        Descriptor::from_bytes(&trailing, limits, &()),
        Err(Error::InvalidEncoding)
    ));
    let mut version = bytes.clone();
    version[4] = 2;
    assert_eq!(
        Descriptor::from_bytes(&version, limits, &()),
        Err(Error::UnsupportedVersion(2))
    );
    let mut tag = bytes.clone();
    tag[5] = 99;
    assert_eq!(
        Descriptor::from_bytes(&tag, limits, &()),
        Err(Error::InvalidEncoding)
    );
    let mut oversized = bytes.clone();
    oversized[6..14].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(Descriptor::from_bytes(&oversized, limits, &()).is_err());
    for limit in [
        DescriptorLimits { bytes: 1, ..limits },
        DescriptorLimits { items: 1, ..limits },
    ] {
        assert!(descriptor.to_bytes(limit, &()).is_err());
        assert!(descriptor.admit(limit, &()).is_err());
        assert!(Descriptor::import(&bytes, limit, &()).is_err());
    }
    let graph_limit = DescriptorLimits {
        events: Limits {
            records: 0,
            ..Limits::default()
        },
        ..limits
    };
    assert!(matches!(
        descriptor.admit(graph_limit, &()),
        Err(Error::Capacity(_))
    ));
    for budget in [0, 1, 5, 20] {
        assert!(matches!(
            Descriptor::import(&bytes, limits, &Stop(Cell::new(budget))),
            Err(Error::Cancelled)
        ));
    }
    assert!(descriptor.admit(limits, &()).is_ok());
}

#[test]
fn syntax_parse_never_asserts_semantic_admission_or_accepts_invalid_orientation() {
    let limits = DescriptorLimits::default();
    let point = world(22, 0, 1);
    let onto = base(&point, &point);
    let product = FibreProduct::new(SpaceId([23; 32]), &onto, &onto, &()).unwrap();
    let mut bytes = erase(&AdmittedDescriptor::Fibre(product))
        .to_bytes(limits, &())
        .unwrap();
    bytes[38] = 2;
    assert_eq!(
        Descriptor::from_bytes(&bytes, limits, &()),
        Err(Error::InvalidEncoding)
    );
    let Descriptor::Map(mut map) = erase(&AdmittedDescriptor::Map(onto.map().clone())) else {
        panic!("map");
    };
    map.source.clear();
    let invalid = Descriptor::Map(map);
    let bytes = invalid.to_bytes(limits, &()).unwrap();
    assert_eq!(
        Descriptor::from_bytes(&bytes, limits, &()).unwrap(),
        invalid
    );
    assert!(matches!(
        Descriptor::import(&bytes, limits, &()),
        Err(Error::InvalidEncoding)
    ));
    let no_faces = Descriptor::Faces {
        identity: SpaceId([24; 32]),
        environments: vec![],
    };
    assert!(matches!(no_faces.admit(limits, &()), Err(Error::NoFaces)));
}
