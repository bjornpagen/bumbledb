use super::{
    ConstraintDescriptor, EnumDescriptor, EnumVariantDescriptor, FieldDescriptor, FieldGeneration,
    ForeignKeyAction, RelationDescriptor, SchemaDescriptor, SchemaFingerprint, ValueType,
};

impl SchemaDescriptor {
    /// Computes the deterministic schema fingerprint.
    pub fn fingerprint(&self) -> SchemaFingerprint {
        SchemaFingerprint(*blake3::hash(&self.canonical_bytes()).as_bytes())
    }

    fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        push_str(&mut out, "bumbledb.schema.v4.set-native-layout");
        push_str(&mut out, &self.name);
        push_u32(&mut out, self.enums.len() as u32);
        for enum_descriptor in &self.enums {
            enum_descriptor.push_canonical(&mut out);
        }
        push_u32(&mut out, self.relations.len() as u32);
        for relation in &self.relations {
            relation.push_canonical(&mut out);
        }
        out
    }
}

impl EnumDescriptor {
    fn push_canonical(&self, out: &mut Vec<u8>) {
        push_str(out, &self.name);
        push_u32(out, self.variants.len() as u32);
        for variant in &self.variants {
            variant.push_canonical(out);
        }
    }
}

impl EnumVariantDescriptor {
    fn push_canonical(&self, out: &mut Vec<u8>) {
        push_str(out, &self.name);
        push_u8(out, self.code);
    }
}

impl RelationDescriptor {
    fn push_canonical(&self, out: &mut Vec<u8>) {
        push_str(out, &self.name);

        push_u32(out, self.fields.len() as u32);
        for field in &self.fields {
            field.push_canonical(out);
        }

        push_u32(out, self.constraints.len() as u32);
        for constraint in &self.constraints {
            constraint.push_canonical(out);
        }
    }
}

impl FieldDescriptor {
    fn push_canonical(&self, out: &mut Vec<u8>) {
        push_str(out, &self.name);
        self.value_type.push_canonical(out);
        self.generation.push_canonical(out);
    }
}

impl FieldGeneration {
    fn push_canonical(self, out: &mut Vec<u8>) {
        push_u8(
            out,
            match self {
                FieldGeneration::None => 0,
                FieldGeneration::SerialSequence => 1,
            },
        );
    }
}

impl ValueType {
    fn push_canonical(&self, out: &mut Vec<u8>) {
        match self {
            ValueType::Bool => push_u8(out, 1),
            ValueType::U64 => push_u8(out, 2),
            ValueType::I64 => push_u8(out, 3),
            ValueType::Enum { name } => {
                push_u8(out, 7);
                push_str(out, name);
            }
            ValueType::String => push_u8(out, 8),
            ValueType::Bytes => push_u8(out, 9),
            ValueType::Serial {
                type_name,
                owning_relation,
            } => {
                push_u8(out, 10);
                push_str(out, type_name);
                push_str(out, owning_relation);
            }
        }
    }
}

impl ConstraintDescriptor {
    fn push_canonical(&self, out: &mut Vec<u8>) {
        match self {
            ConstraintDescriptor::Unique { name, fields } => {
                push_u8(out, 1);
                push_str(out, name);
                push_string_list(out, fields);
            }
            ConstraintDescriptor::ForeignKey {
                name,
                fields,
                target_relation,
                target_constraint,
                on_delete,
            } => {
                push_u8(out, 2);
                push_str(out, name);
                push_string_list(out, fields);
                push_str(out, target_relation);
                push_str(out, target_constraint);
                on_delete.push_canonical(out);
            }
        }
    }
}

impl ForeignKeyAction {
    fn push_canonical(self, out: &mut Vec<u8>) {
        match self {
            ForeignKeyAction::Restrict => push_u8(out, 1),
        }
    }
}

fn push_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn push_str(out: &mut Vec<u8>, value: &str) {
    push_u32(out, value.len() as u32);
    out.extend_from_slice(value.as_bytes());
}

fn push_string_list(out: &mut Vec<u8>, values: &[String]) {
    push_u32(out, values.len() as u32);
    for value in values {
        push_str(out, value);
    }
}
