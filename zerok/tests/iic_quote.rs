use iic_quote::iic_quote;
use ir::{IrChunk, ToIIC};
use zerok::annotator::{EnvEntry, Kind};


fn iic(chunks: Vec<IrChunk<EnvEntry, Kind>>) -> Vec<String>
{
    chunks.iter().map(|chunk| chunk.to_iic()).collect()
}


#[test]
fn memory()
{
    let base = 7_usize;
    let offset = 2_u32;
    let ref_u32 = Kind::Ref(&Kind::U32);

    assert_eq!(
        iic(iic_quote! {
            R(11, ref_u32) = <mk, 4>;
            R(11, ref_u32) = <mk, R(3)>;

            R(10, Kind::U32) = R(base)<offset>;
            R(10, Kind::U32) = R(base)<R(3)>;
            R(10, Kind::U32) = R(base)<offset + 1>;

            R(base, ref_u32)<0> = R(10);
            R(base, ref_u32)<R(3)> = 5;
            R(base, ref_u32)<offset * 2> = R(10)
        }),
        [
            "    R11: &u32 = mk[] 4",
            "    R11: &u32 = mk[] R3",
            "    R10: u32 = R7[2]",
            "    R10: u32 = R7[R3]",
            "    R10: u32 = R7[3]",
            "    R7: &u32[0] = R10",
            "    R7: &u32[R3] = 5",
            "    R7: &u32[4] = R10",
        ]
    );
}


#[test]
fn comparisons_are_not_memory_accesses()
{
    assert_eq!(
        iic(iic_quote! {
            R(1, Kind::Bool) = R(2) < R(3);
            R(1, Kind::Bool) = R(2) > R(3);
            R(1, Kind::Bool) = R(2) <= 3;
            R(1, Kind::Bool) = 2 < 3
        }),
        [
            "    R1: bool = R2 < R3",
            "    R1: bool = R2 > R3",
            "    R1: bool = R2 <= 3",
            "    R1: bool = 2 < 3",
        ]
    );
}


#[test]
fn global_registers()
{
    let global = 3_usize;
    let ref_u64 = Kind::Ref(&Kind::U64);

    assert_eq!(
        iic(iic_quote! {
            // As target
            G(global, ref_u64) = <mk, 3>;
            G(global, ref_u64)<0> = R(5);
            G(1, Kind::U64) = 7;

            // As operand
            R(10, Kind::U64) = G(global)<0>;
            R(10, Kind::U64) = G(1) + R(10);
            R(10, Kind::U64) = R(10) * G(global + 1);
            R(2, ref_u64)<G(1)> = G(1);
            if G(1) goto "done"
        }),
        [
            "    G3: &u64 = mk[] 3",
            "    G3: &u64[0] = R5",
            "    G1: u64 = 7",
            "    R10: u64 = G3[0]",
            "    R10: u64 = G1 + R10",
            "    R10: u64 = R10 * G4",
            "    R2: &u64[G1] = G1",
            "    if G1 goto done",
        ]
    );
}


#[test]
fn labels_as_values()
{
    let rt_data_reg = 256_usize;
    let rt_data = Kind::U64.referenced();
    let ir_name = String::from("zk_add");

    assert_eq!(
        iic(iic_quote! {
            G(rt_data_reg, rt_data) = <mk, 3>;
            G(rt_data_reg, rt_data)<0> = EnvEntry::Label(ir_name);
            G(rt_data_reg, rt_data)<1> = 0;
            G(rt_data_reg, rt_data)<2> = 0;
            R(1, Kind::U64) = EnvEntry::Label("zk_main".into())
        }),
        [
            "    G256: &u64 = mk[] 3",
            "    G256: &u64[0] = zk_add",
            "    G256: &u64[1] = 0",
            "    G256: &u64[2] = 0",
            "    R1: u64 = zk_main",
        ]
    );
}
