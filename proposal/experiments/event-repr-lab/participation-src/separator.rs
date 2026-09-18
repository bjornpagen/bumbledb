//! A conservative syntactic separator certificate on the actual normalized IR.
//! It proves separation of two positive branch blocks after fixing the keys.
//! It does not discover dependencies beyond variable incidence.
use crate::ir::{VarId,normalize::{NormalizedQuery,OccId,Role}};
use std::collections::BTreeSet;

#[derive(Clone,Debug)]
pub struct Certificate { keys:BTreeSet<VarId>, blocks:[BTreeSet<OccId>;2] }
impl Certificate {
    pub fn derive(q:&NormalizedQuery,blocks:[BTreeSet<OccId>;2],keys:BTreeSet<VarId>)->Result<Self,&'static str> {
        if q.dead.is_some() || !q.residuals.is_empty() || !q.word_residuals.is_empty()
            || !q.allen_residuals.is_empty() || !q.anti_probes.is_empty() {
            return Err("separator does not cover global conditions");
        }
        if blocks.iter().any(BTreeSet::is_empty) || !blocks[0].is_disjoint(&blocks[1]) {
            return Err("branches must be a disjoint nonempty occurrence partition");
        }
        let roster:BTreeSet<_>=q.occurrences.iter().map(|o|o.occ_id).collect();
        if roster.len()!=q.occurrences.len() || roster!=blocks[0].union(&blocks[1]).copied().collect() {
            return Err("occurrence partition does not match query");
        }
        let mut variables:[BTreeSet<VarId>;2]=std::array::from_fn(|_|BTreeSet::new());
        for o in &q.occurrences {
            // Local filters could be admitted with their own incidence check.
            // Reject them and derived/point occurrences in this initial certificate.
            if o.role!=Role::Positive || o.bind.edb().is_none() || !o.filters.is_empty() || !o.point_vars.is_empty() {
                return Err("separator requires pure positive stored occurrences");
            }
            let b=usize::from(blocks[1].contains(&o.occ_id));
            variables[b].extend(o.vars.iter().map(|(_,v)|*v));
        }
        if variables[0].intersection(&variables[1]).copied().collect::<BTreeSet<_>>()!=keys {
            return Err("separator is not exactly the shared scalar variables");
        }
        Ok(Self{keys,blocks})
    }
    pub fn key_count(&self)->usize {self.keys.len()}
}

pub fn verify() {
    use crate::ir::normalize::{OccBind,Occurrence,SlotWidth};
    use bumbledb_theory::schema::{RelationId,FieldId};
    let mut checked=0;
    for left in 0..16u32 {for right in 0..16u32 {for keys in 0..16u32 {
        let vs=|mask:u32|(0..4).filter(move |&v|mask>>v&1!=0).map(|v|VarId(v)).collect::<BTreeSet<_>>();
        let occurrences=[left,right].into_iter().enumerate().map(|(i,mask)|Occurrence{
            occ_id:OccId(i as u16),bind:OccBind::Edb(RelationId(i as u32)),role:Role::Positive,
            vars:vs(mask).into_iter().enumerate().map(|(j,v)|(FieldId(j as u16),v)).collect(),
            filters:vec![],point_vars:vec![],
        }).collect();
        let mut q=NormalizedQuery{occurrences,residuals:vec![],word_residuals:vec![],allen_residuals:vec![],
            anti_probes:vec![],slot_widths:vs(left|right).into_iter().map(|v|(v,SlotWidth::ONE)).collect(),dead:None};
        let blocks=||[BTreeSet::from([OccId(0)]),BTreeSet::from([OccId(1)])];
        assert_eq!(Certificate::derive(&q,blocks(),vs(keys)).is_ok(),left&right==keys);checked+=1;
        q.dead=Some("false guard".into());assert!(Certificate::derive(&q,blocks(),vs(keys)).is_err());checked+=1;
        q.dead=None;q.occurrences[1].role=Role::Negated;assert!(Certificate::derive(&q,blocks(),vs(keys)).is_err());checked+=1;
    }}}
    println!("EVENT_LAB {{\"kind\":\"separator_verification\",\"cases\":{checked},\"passed\":true}}");
}
