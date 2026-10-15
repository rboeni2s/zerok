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
