// Wire
// Copyright (C) 2018 Wire Swiss GmbH
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see http://www.gnu.org/licenses/.

use codec::*;
use std::ops::Range;

pub fn log2(n: usize) -> usize {
    let mut r = 0;
    let mut m = n;
    while m > 1 {
        m >>= 1;
        r += 1;
    }
    r
}

pub fn pow2(n: usize) -> usize {
    match n {
        0 => 1,
        _ => 2 << (n - 1),
    }
}

pub fn level(n: usize) -> usize {
    if (n & 0x01) == 0 {
        return 0;
    }
    let mut k = 0;
    while ((n >> k) & 0x01) == 1 {
        k += 1;
    }
    k
}

pub fn node_width(n: usize) -> usize {
    2 * (n - 1) + 1
}

pub fn assert_in_range(x: usize, n: usize) {
    if x > node_width(n) {
        panic!("node index out of range ({} > {})", x, n);
    }
}

pub fn root(n: usize) -> usize {
    let w = node_width(n);
    (1 << log2(w)) - 1
}

pub fn left(x: usize) -> usize {
    if level(x) == 0 {
        return x;
    }
    x ^ (0x01 << (level(x) - 1))
}

pub fn right(x: usize, n: usize) -> usize {
    assert_in_range(x, n);
    if level(x) == 0 {
        return x;
    }
    let mut r = x ^ (0x03 << (level(x) - 1));
    while r >= node_width(n) {
        r = left(r);
    }
    r
}

pub fn parent_step(x: usize) -> usize {
    let k = level(x);
    (x | (1 << k)) & !(1 << (k + 1))
}

pub fn parent(x: usize, n: usize) -> usize {
    assert_in_range(x, n);

    if x == root(n) {
        return x;
    }
    let mut p = parent_step(x);
    while p >= node_width(n) {
        p = parent_step(p);
    }
    p
}

pub fn sibling(x: usize, n: usize) -> usize {
    assert_in_range(x, n);

    let p = parent(x, n);
    if x < p {
        return right(p, n);
    } else if x > p {
        return left(p);
    }
    // root's sibling is itself
    p
}

// Ordered from leaf to root
// Includes leaf, but not root
pub fn dirpath(x: usize, n: usize) -> Vec<usize> {
    assert_in_range(x, n);
    if x == root(n) {
        return Vec::new();
    }
    let mut dirpath = vec![x];
    let mut node_parent = parent(x, n);
    let root = root(n);
    while node_parent != root {
        dirpath.push(node_parent);
        node_parent = parent(node_parent, n);
    }
    dirpath
}

// Ordered from leaf to root
pub fn copath(x: usize, n: usize) -> Vec<usize> {
    dirpath(x, n).iter().map(|&x| sibling(x, n)).collect()
}

pub fn leaves(n: usize) -> Vec<usize> {
    Range { start: 0, end: n }.map(|x| 2 * x).collect()
}

pub fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut hex = String::new();
    for b in bytes {
        hex += &format!("{:02X}", *b);
    }
    hex
}

pub fn hex_to_bytes(hex: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    for i in 0..(hex.len() / 2) {
        let b = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap();
        bytes.push(b);
    }
    bytes
}

#[derive(Clone, Copy)]
pub enum FunctionType {
    OneArg(fn(usize) -> usize),
    TwoArgs(fn(usize, usize) -> usize),
    TwoArgsPath(fn(usize, usize) -> Vec<usize>),
}

pub enum ReturnType {
    Primitive(Vec<usize>),
    Vector(Vec<Vec<usize>>),
}

pub fn gen_vector(range_start: usize, range_end: usize, size: usize, ft: FunctionType) -> Vec<u8> {
    let range = Range {
        start: range_start,
        end: range_end,
    };
    let mut test_vector: Vec<u8> = Vec::new();
    let mut test_vector_2d: Vec<Vec<u8>> = Vec::new();
    for i in range {
        match ft {
            FunctionType::OneArg(f) => {
                test_vector.push(f(i) as u8);
            }
            FunctionType::TwoArgs(f) => {
                test_vector.push(f(i, size) as u8);
            }
            FunctionType::TwoArgsPath(f) => {
                let sub_vector_usize = f(i, size);
                let mut sub_vector_u8 = Vec::new();
                sub_vector_usize
                    .iter()
                    .for_each(|&x| sub_vector_u8.push(x as u8));
                test_vector_2d.push(sub_vector_u8);
            }
        }
    }

    let mut buffer = Vec::new();
    let num_elements = range_end - range_start + 1;
    (num_elements as u8).encode(&mut buffer);

    match ft {
        FunctionType::OneArg(_) => {
            encode_vec_u8(&mut buffer, &test_vector);
        }
        FunctionType::TwoArgs(_) => {
            encode_vec_u8(&mut buffer, &test_vector);
        }
        FunctionType::TwoArgsPath(_) => {
            for e in test_vector_2d.iter_mut() {
                (e.len() as u8).encode(&mut buffer);
                encode_vec_u8(&mut buffer, e);
            }
        }
    }
    buffer
}

pub fn read_vector(rt: &ReturnType, buffer: &[u8]) -> ReturnType {
    let mut vector = Vec::new();
    let mut vector2d = Vec::new();
    let mut cursor = Cursor::new(buffer);

    match *rt {
        ReturnType::Primitive(_) => {
            let vector_usize: Vec<u8> = decode_vec_u8(&mut cursor).unwrap();
            vector_usize.iter().for_each(|&x| vector.push(x as usize));
            ReturnType::Primitive(vector)
        }
        ReturnType::Vector(_) => {
            let size = cursor.take(1).unwrap()[0];
            for _ in 0..size {
                let mut sub_vector = Vec::new();
                let sub_vector_usize: Vec<u8> = decode_vec_u8(&mut cursor).unwrap();
                sub_vector_usize
                    .iter()
                    .for_each(|&x| sub_vector.push(x as usize));
                vector2d.push(sub_vector);
            }
            ReturnType::Vector(vector2d)
        }
    }
}

#[test]
fn print_test_vectors() {
    let size = 255;
    println!(
        "Test vector for root() with size {}:\n{}",
        size,
        bytes_to_hex(&gen_vector(1, size, size, FunctionType::OneArg(root),))
    );
    println!(
        "Test vector for level() with size {}:\n{}",
        size,
        bytes_to_hex(&gen_vector(0, size - 1, size, FunctionType::OneArg(level),))
    );
    println!(
        "Test vector for node_width() with size {}:\n{}",
        size,
        bytes_to_hex(&gen_vector(1, size, size, FunctionType::OneArg(node_width),))
    );
    println!(
        "Test vector for left() with size {}:\n{}",
        size,
        bytes_to_hex(&gen_vector(0, size - 1, size, FunctionType::OneArg(left),))
    );
    println!(
        "Test vector for parent_step() with size {}:\n{}",
        size,
        bytes_to_hex(&gen_vector(
            0,
            size - 1,
            size,
            FunctionType::OneArg(parent_step),
        ))
    );
    println!(
        "Test vector for right() with size {}:\n{}",
        size,
        bytes_to_hex(&gen_vector(0, size - 1, size, FunctionType::TwoArgs(right),))
    );
    println!(
        "Test vector for parent() with size {}:\n{}",
        size,
        bytes_to_hex(&gen_vector(
            0,
            size - 1,
            size,
            FunctionType::TwoArgs(parent),
        ))
    );
    println!(
        "Test vector for sibling() with size {}:\n{}",
        size,
        bytes_to_hex(&gen_vector(
            0,
            size - 1,
            size,
            FunctionType::TwoArgs(sibling),
        ))
    );
    println!(
        "Test vector for dirpath() with size {}:\n{}",
        size,
        bytes_to_hex(&gen_vector(
            0,
            size - 1,
            size,
            FunctionType::TwoArgsPath(dirpath),
        ))
    );
    println!(
        "Test vector for copath() with size {}:\n{}",
        size,
        bytes_to_hex(&gen_vector(
            0,
            size - 1,
            size,
            FunctionType::TwoArgsPath(copath),
        ))
    );
}

#[test]
fn compare_test_vectors() {
    fn test_vector(
        test_vector_hex: &str,
        range_start: usize,
        size: usize,
        ft: FunctionType,
    ) -> bool {
        let test_vector = hex_to_bytes(test_vector_hex);
        let gen_vector = gen_vector(range_start, size + range_start - 1, size, ft);
        assert_eq!(gen_vector, test_vector);
        gen_vector == test_vector
    }

    let size = 255;
    let root_hex = "FFFE00010303070707070F0F0F0F0F0F0F0F1F1F1F1F1F1F1F1F1F1F1F1F1F1F1F1F3F3F3F3F3F3F3F3F3F3F3F3F3F3F3F3F3F3F3F3F3F3F3F3F3F3F3F3F3F3F3F3F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7F7FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF";
    assert!(test_vector(root_hex, 1, size, FunctionType::OneArg(root)));
    let level_hex = "FFFE0001000200010003000100020001000400010002000100030001000200010005000100020001000300010002000100040001000200010003000100020001000600010002000100030001000200010004000100020001000300010002000100050001000200010003000100020001000400010002000100030001000200010007000100020001000300010002000100040001000200010003000100020001000500010002000100030001000200010004000100020001000300010002000100060001000200010003000100020001000400010002000100030001000200010005000100020001000300010002000100040001000200010003000100020001";
    assert!(test_vector(level_hex, 0, size, FunctionType::OneArg(level)));
    let node_width_hex = "FFFE01030507090B0D0F11131517191B1D1F21232527292B2D2F31333537393B3D3F41434547494B4D4F51535557595B5D5F61636567696B6D6F71737577797B7D7F81838587898B8D8F91939597999B9D9FA1A3A5A7A9ABADAFB1B3B5B7B9BBBDBFC1C3C5C7C9CBCDCFD1D3D5D7D9DBDDDFE1E3E5E7E9EBEDEFF1F3F5F7F9FBFDFF01030507090B0D0F11131517191B1D1F21232527292B2D2F31333537393B3D3F41434547494B4D4F51535557595B5D5F61636567696B6D6F71737577797B7D7F81838587898B8D8F91939597999B9D9FA1A3A5A7A9ABADAFB1B3B5B7B9BBBDBFC1C3C5C7C9CBCDCFD1D3D5D7D9DBDDDFE1E3E5E7E9EBEDEFF1F3F5F7F9FB";
    assert!(test_vector(
        node_width_hex,
        1,
        size,
        FunctionType::OneArg(node_width)
    ));
    let left_hex = "FFFE000002010404060308080A090C0C0E07101012111414161318181A191C1C1E0F202022212424262328282A292C2C2E27303032313434363338383A393C3C3E1F404042414444464348484A494C4C4E47505052515454565358585A595C5C5E4F606062616464666368686A696C6C6E67707072717474767378787A797C7C7E3F808082818484868388888A898C8C8E87909092919494969398989A999C9C9E8FA0A0A2A1A4A4A6A3A8A8AAA9ACACAEA7B0B0B2B1B4B4B6B3B8B8BAB9BCBCBE9FC0C0C2C1C4C4C6C3C8C8CAC9CCCCCEC7D0D0D2D1D4D4D6D3D8D8DAD9DCDCDECFE0E0E2E1E4E4E6E3E8E8EAE9ECECEEE7F0F0F2F1F4F4F6F3F8F8FAF9FCFC";
    assert!(test_vector(left_hex, 0, size, FunctionType::OneArg(left)));
    let parent_step_hex = "FFFE010301070503050F090B09070D0B0D1F111311171513150F191B19171D1B1D3F212321272523252F292B29272D2B2D1F313331373533352F393B39373D3B3D7F414341474543454F494B49474D4B4D5F515351575553554F595B59575D5B5D3F616361676563656F696B69676D6B6D5F717371777573756F797B79777D7B7DFF818381878583858F898B89878D8B8D9F919391979593958F999B99979D9B9DBFA1A3A1A7A5A3A5AFA9ABA9A7ADABAD9FB1B3B1B7B5B3B5AFB9BBB9B7BDBBBD7FC1C3C1C7C5C3C5CFC9CBC9C7CDCBCDDFD1D3D1D7D5D3D5CFD9DBD9D7DDDBDDBFE1E3E1E7E5E3E5EFE9EBE9E7EDEBEDDFF1F3F1F7F5F3F5EFF9FBF9F7FDFB";
    assert!(test_vector(
        parent_step_hex,
        0,
        size,
        FunctionType::OneArg(parent_step)
    ));
    let right_hex = "FFFE000202050406060B080A0A0D0C0E0E17101212151416161B181A1A1D1C1E1E2F202222252426262B282A2A2D2C2E2E37303232353436363B383A3A3D3C3E3E5F404242454446464B484A4A4D4C4E4E57505252555456565B585A5A5D5C5E5E6F606262656466666B686A6A6D6C6E6E77707272757476767B787A7A7D7C7E7EBF808282858486868B888A8A8D8C8E8E97909292959496969B989A9A9D9C9E9EAFA0A2A2A5A4A6A6ABA8AAAAADACAEAEB7B0B2B2B5B4B6B6BBB8BABABDBCBEBEDFC0C2C2C5C4C6C6CBC8CACACDCCCECED7D0D2D2D5D4D6D6DBD8DADADDDCDEDEEFE0E2E2E5E4E6E6EBE8EAEAEDECEEEEF7F0F2F2F5F4F6F6FBF8FAFAFDFCFE";
    assert!(test_vector(
        right_hex,
        0,
        size,
        FunctionType::TwoArgs(right)
    ));
    let parent_hex = "FFFE010301070503050F090B09070D0B0D1F111311171513150F191B19171D1B1D3F212321272523252F292B29272D2B2D1F313331373533352F393B39373D3B3D7F414341474543454F494B49474D4B4D5F515351575553554F595B59575D5B5D3F616361676563656F696B69676D6B6D5F717371777573756F797B79777D7B7DFF818381878583858F898B89878D8B8D9F919391979593958F999B99979D9B9DBFA1A3A1A7A5A3A5AFA9ABA9A7ADABAD9FB1B3B1B7B5B3B5AFB9BBB9B7BDBBBD7FC1C3C1C7C5C3C5CFC9CBC9C7CDCBCDDFD1D3D1D7D5D3D5CFD9DBD9D7DDDBDDBFE1E3E1E7E5E3E5EFE9EBE9E7EDEBEDDFF1F3F1F7F5F3F5EFF9FBF9F7FDFB";
    assert!(test_vector(
        parent_hex,
        0,
        size,
        FunctionType::TwoArgs(parent)
    ));
    let sibling_hex = "FFFE0205000B060104170A0D08030E090C2F1215101B161114071A1D18131E191C5F2225202B262124372A2D28232E292C0F3235303B363134273A3D38333E393CBF4245404B464144574A4D48434E494C6F5255505B565154475A5D58535E595C1F6265606B666164776A6D68636E696C4F7275707B767174677A7D78737E797C7F8285808B868184978A8D88838E898CAF9295909B969194879A9D98939E999CDFA2A5A0ABA6A1A4B7AAADA8A3AEA9AC8FB2B5B0BBB6B1B4A7BABDB8B3BEB9BC3FC2C5C0CBC6C1C4D7CACDC8C3CEC9CCEFD2D5D0DBD6D1D4C7DADDD8D3DED9DC9FE2E5E0EBE6E1E4F7EAEDE8E3EEE9ECCFF2F5F0FBF6F1F4E7FAFDF8F3FEF9";
    assert!(test_vector(
        sibling_hex,
        0,
        size,
        FunctionType::TwoArgs(sibling)
    ));
    let dirpath_hex = "FF0808000103070F1F3F7F07070103070F1F3F7F0808020103070F1F3F7F060603070F1F3F7F0808040503070F1F3F7F07070503070F1F3F7F0808060503070F1F3F7F0505070F1F3F7F080808090B070F1F3F7F0707090B070F1F3F7F08080A090B070F1F3F7F06060B070F1F3F7F08080C0D0B070F1F3F7F07070D0B070F1F3F7F08080E0D0B070F1F3F7F04040F1F3F7F0808101113170F1F3F7F07071113170F1F3F7F0808121113170F1F3F7F060613170F1F3F7F0808141513170F1F3F7F07071513170F1F3F7F0808161513170F1F3F7F0505170F1F3F7F080818191B170F1F3F7F0707191B170F1F3F7F08081A191B170F1F3F7F06061B170F1F3F7F08081C1D1B170F1F3F7F07071D1B170F1F3F7F08081E1D1B170F1F3F7F03031F3F7F0808202123272F1F3F7F07072123272F1F3F7F0808222123272F1F3F7F060623272F1F3F7F0808242523272F1F3F7F07072523272F1F3F7F0808262523272F1F3F7F0505272F1F3F7F080828292B272F1F3F7F0707292B272F1F3F7F08082A292B272F1F3F7F06062B272F1F3F7F08082C2D2B272F1F3F7F07072D2B272F1F3F7F08082E2D2B272F1F3F7F04042F1F3F7F0808303133372F1F3F7F07073133372F1F3F7F0808323133372F1F3F7F060633372F1F3F7F0808343533372F1F3F7F07073533372F1F3F7F0808363533372F1F3F7F0505372F1F3F7F080838393B372F1F3F7F0707393B372F1F3F7F08083A393B372F1F3F7F06063B372F1F3F7F08083C3D3B372F1F3F7F07073D3B372F1F3F7F08083E3D3B372F1F3F7F02023F7F0808404143474F5F3F7F07074143474F5F3F7F0808424143474F5F3F7F060643474F5F3F7F0808444543474F5F3F7F07074543474F5F3F7F0808464543474F5F3F7F0505474F5F3F7F080848494B474F5F3F7F0707494B474F5F3F7F08084A494B474F5F3F7F06064B474F5F3F7F08084C4D4B474F5F3F7F07074D4B474F5F3F7F08084E4D4B474F5F3F7F04044F5F3F7F0808505153574F5F3F7F07075153574F5F3F7F0808525153574F5F3F7F060653574F5F3F7F0808545553574F5F3F7F07075553574F5F3F7F0808565553574F5F3F7F0505574F5F3F7F080858595B574F5F3F7F0707595B574F5F3F7F08085A595B574F5F3F7F06065B574F5F3F7F08085C5D5B574F5F3F7F07075D5B574F5F3F7F08085E5D5B574F5F3F7F03035F3F7F0808606163676F5F3F7F07076163676F5F3F7F0808626163676F5F3F7F060663676F5F3F7F0808646563676F5F3F7F07076563676F5F3F7F0808666563676F5F3F7F0505676F5F3F7F080868696B676F5F3F7F0707696B676F5F3F7F08086A696B676F5F3F7F06066B676F5F3F7F08086C6D6B676F5F3F7F07076D6B676F5F3F7F08086E6D6B676F5F3F7F04046F5F3F7F0808707173776F5F3F7F07077173776F5F3F7F0808727173776F5F3F7F060673776F5F3F7F0808747573776F5F3F7F07077573776F5F3F7F0808767573776F5F3F7F0505776F5F3F7F080878797B776F5F3F7F0707797B776F5F3F7F08087A797B776F5F3F7F06067B776F5F3F7F08087C7D7B776F5F3F7F07077D7B776F5F3F7F08087E7D7B776F5F3F7F01017F0808808183878F9FBF7F07078183878F9FBF7F0808828183878F9FBF7F060683878F9FBF7F0808848583878F9FBF7F07078583878F9FBF7F0808868583878F9FBF7F0505878F9FBF7F080888898B878F9FBF7F0707898B878F9FBF7F08088A898B878F9FBF7F06068B878F9FBF7F08088C8D8B878F9FBF7F07078D8B878F9FBF7F08088E8D8B878F9FBF7F04048F9FBF7F0808909193978F9FBF7F07079193978F9FBF7F0808929193978F9FBF7F060693978F9FBF7F0808949593978F9FBF7F07079593978F9FBF7F0808969593978F9FBF7F0505978F9FBF7F080898999B978F9FBF7F0707999B978F9FBF7F08089A999B978F9FBF7F06069B978F9FBF7F08089C9D9B978F9FBF7F07079D9B978F9FBF7F08089E9D9B978F9FBF7F03039FBF7F0808A0A1A3A7AF9FBF7F0707A1A3A7AF9FBF7F0808A2A1A3A7AF9FBF7F0606A3A7AF9FBF7F0808A4A5A3A7AF9FBF7F0707A5A3A7AF9FBF7F0808A6A5A3A7AF9FBF7F0505A7AF9FBF7F0808A8A9ABA7AF9FBF7F0707A9ABA7AF9FBF7F0808AAA9ABA7AF9FBF7F0606ABA7AF9FBF7F0808ACADABA7AF9FBF7F0707ADABA7AF9FBF7F0808AEADABA7AF9FBF7F0404AF9FBF7F0808B0B1B3B7AF9FBF7F0707B1B3B7AF9FBF7F0808B2B1B3B7AF9FBF7F0606B3B7AF9FBF7F0808B4B5B3B7AF9FBF7F0707B5B3B7AF9FBF7F0808B6B5B3B7AF9FBF7F0505B7AF9FBF7F0808B8B9BBB7AF9FBF7F0707B9BBB7AF9FBF7F0808BAB9BBB7AF9FBF7F0606BBB7AF9FBF7F0808BCBDBBB7AF9FBF7F0707BDBBB7AF9FBF7F0808BEBDBBB7AF9FBF7F0202BF7F0808C0C1C3C7CFDFBF7F0707C1C3C7CFDFBF7F0808C2C1C3C7CFDFBF7F0606C3C7CFDFBF7F0808C4C5C3C7CFDFBF7F0707C5C3C7CFDFBF7F0808C6C5C3C7CFDFBF7F0505C7CFDFBF7F0808C8C9CBC7CFDFBF7F0707C9CBC7CFDFBF7F0808CAC9CBC7CFDFBF7F0606CBC7CFDFBF7F0808CCCDCBC7CFDFBF7F0707CDCBC7CFDFBF7F0808CECDCBC7CFDFBF7F0404CFDFBF7F0808D0D1D3D7CFDFBF7F0707D1D3D7CFDFBF7F0808D2D1D3D7CFDFBF7F0606D3D7CFDFBF7F0808D4D5D3D7CFDFBF7F0707D5D3D7CFDFBF7F0808D6D5D3D7CFDFBF7F0505D7CFDFBF7F0808D8D9DBD7CFDFBF7F0707D9DBD7CFDFBF7F0808DAD9DBD7CFDFBF7F0606DBD7CFDFBF7F0808DCDDDBD7CFDFBF7F0707DDDBD7CFDFBF7F0808DEDDDBD7CFDFBF7F0303DFBF7F0808E0E1E3E7EFDFBF7F0707E1E3E7EFDFBF7F0808E2E1E3E7EFDFBF7F0606E3E7EFDFBF7F0808E4E5E3E7EFDFBF7F0707E5E3E7EFDFBF7F0808E6E5E3E7EFDFBF7F0505E7EFDFBF7F0808E8E9EBE7EFDFBF7F0707E9EBE7EFDFBF7F0808EAE9EBE7EFDFBF7F0606EBE7EFDFBF7F0808ECEDEBE7EFDFBF7F0707EDEBE7EFDFBF7F0808EEEDEBE7EFDFBF7F0404EFDFBF7F0808F0F1F3F7EFDFBF7F0707F1F3F7EFDFBF7F0808F2F1F3F7EFDFBF7F0606F3F7EFDFBF7F0808F4F5F3F7EFDFBF7F0707F5F3F7EFDFBF7F0808F6F5F3F7EFDFBF7F0505F7EFDFBF7F0808F8F9FBF7EFDFBF7F0707F9FBF7EFDFBF7F0808FAF9FBF7EFDFBF7F0606FBF7EFDFBF7F0808FCFDFBF7EFDFBF7F0707FDFBF7EFDFBF7F";
    assert!(test_vector(
        dirpath_hex,
        0,
        size,
        FunctionType::TwoArgsPath(dirpath)
    ));
    let copath_hex = "FF080802050B172F5FBF7F0707050B172F5FBF7F080800050B172F5FBF7F06060B172F5FBF7F080806010B172F5FBF7F0707010B172F5FBF7F080804010B172F5FBF7F0505172F5FBF7F08080A0D03172F5FBF7F07070D03172F5FBF7F0808080D03172F5FBF7F060603172F5FBF7F08080E0903172F5FBF7F07070903172F5FBF7F08080C0903172F5FBF7F04042F5FBF7F080812151B072F5FBF7F0707151B072F5FBF7F080810151B072F5FBF7F06061B072F5FBF7F080816111B072F5FBF7F0707111B072F5FBF7F080814111B072F5FBF7F0505072F5FBF7F08081A1D13072F5FBF7F07071D13072F5FBF7F0808181D13072F5FBF7F060613072F5FBF7F08081E1913072F5FBF7F07071913072F5FBF7F08081C1913072F5FBF7F03035FBF7F080822252B370F5FBF7F0707252B370F5FBF7F080820252B370F5FBF7F06062B370F5FBF7F080826212B370F5FBF7F0707212B370F5FBF7F080824212B370F5FBF7F0505370F5FBF7F08082A2D23370F5FBF7F07072D23370F5FBF7F0808282D23370F5FBF7F060623370F5FBF7F08082E2923370F5FBF7F07072923370F5FBF7F08082C2923370F5FBF7F04040F5FBF7F080832353B270F5FBF7F0707353B270F5FBF7F080830353B270F5FBF7F06063B270F5FBF7F080836313B270F5FBF7F0707313B270F5FBF7F080834313B270F5FBF7F0505270F5FBF7F08083A3D33270F5FBF7F07073D33270F5FBF7F0808383D33270F5FBF7F060633270F5FBF7F08083E3933270F5FBF7F07073933270F5FBF7F08083C3933270F5FBF7F0202BF7F080842454B576F1FBF7F0707454B576F1FBF7F080840454B576F1FBF7F06064B576F1FBF7F080846414B576F1FBF7F0707414B576F1FBF7F080844414B576F1FBF7F0505576F1FBF7F08084A4D43576F1FBF7F07074D43576F1FBF7F0808484D43576F1FBF7F060643576F1FBF7F08084E4943576F1FBF7F07074943576F1FBF7F08084C4943576F1FBF7F04046F1FBF7F080852555B476F1FBF7F0707555B476F1FBF7F080850555B476F1FBF7F06065B476F1FBF7F080856515B476F1FBF7F0707515B476F1FBF7F080854515B476F1FBF7F0505476F1FBF7F08085A5D53476F1FBF7F07075D53476F1FBF7F0808585D53476F1FBF7F060653476F1FBF7F08085E5953476F1FBF7F07075953476F1FBF7F08085C5953476F1FBF7F03031FBF7F080862656B774F1FBF7F0707656B774F1FBF7F080860656B774F1FBF7F06066B774F1FBF7F080866616B774F1FBF7F0707616B774F1FBF7F080864616B774F1FBF7F0505774F1FBF7F08086A6D63774F1FBF7F07076D63774F1FBF7F0808686D63774F1FBF7F060663774F1FBF7F08086E6963774F1FBF7F07076963774F1FBF7F08086C6963774F1FBF7F04044F1FBF7F080872757B674F1FBF7F0707757B674F1FBF7F080870757B674F1FBF7F06067B674F1FBF7F080876717B674F1FBF7F0707717B674F1FBF7F080874717B674F1FBF7F0505674F1FBF7F08087A7D73674F1FBF7F07077D73674F1FBF7F0808787D73674F1FBF7F060673674F1FBF7F08087E7973674F1FBF7F07077973674F1FBF7F08087C7973674F1FBF7F01017F080882858B97AFDF3F7F0707858B97AFDF3F7F080880858B97AFDF3F7F06068B97AFDF3F7F080886818B97AFDF3F7F0707818B97AFDF3F7F080884818B97AFDF3F7F050597AFDF3F7F08088A8D8397AFDF3F7F07078D8397AFDF3F7F0808888D8397AFDF3F7F06068397AFDF3F7F08088E898397AFDF3F7F0707898397AFDF3F7F08088C898397AFDF3F7F0404AFDF3F7F080892959B87AFDF3F7F0707959B87AFDF3F7F080890959B87AFDF3F7F06069B87AFDF3F7F080896919B87AFDF3F7F0707919B87AFDF3F7F080894919B87AFDF3F7F050587AFDF3F7F08089A9D9387AFDF3F7F07079D9387AFDF3F7F0808989D9387AFDF3F7F06069387AFDF3F7F08089E999387AFDF3F7F0707999387AFDF3F7F08089C999387AFDF3F7F0303DF3F7F0808A2A5ABB78FDF3F7F0707A5ABB78FDF3F7F0808A0A5ABB78FDF3F7F0606ABB78FDF3F7F0808A6A1ABB78FDF3F7F0707A1ABB78FDF3F7F0808A4A1ABB78FDF3F7F0505B78FDF3F7F0808AAADA3B78FDF3F7F0707ADA3B78FDF3F7F0808A8ADA3B78FDF3F7F0606A3B78FDF3F7F0808AEA9A3B78FDF3F7F0707A9A3B78FDF3F7F0808ACA9A3B78FDF3F7F04048FDF3F7F0808B2B5BBA78FDF3F7F0707B5BBA78FDF3F7F0808B0B5BBA78FDF3F7F0606BBA78FDF3F7F0808B6B1BBA78FDF3F7F0707B1BBA78FDF3F7F0808B4B1BBA78FDF3F7F0505A78FDF3F7F0808BABDB3A78FDF3F7F0707BDB3A78FDF3F7F0808B8BDB3A78FDF3F7F0606B3A78FDF3F7F0808BEB9B3A78FDF3F7F0707B9B3A78FDF3F7F0808BCB9B3A78FDF3F7F02023F7F0808C2C5CBD7EF9F3F7F0707C5CBD7EF9F3F7F0808C0C5CBD7EF9F3F7F0606CBD7EF9F3F7F0808C6C1CBD7EF9F3F7F0707C1CBD7EF9F3F7F0808C4C1CBD7EF9F3F7F0505D7EF9F3F7F0808CACDC3D7EF9F3F7F0707CDC3D7EF9F3F7F0808C8CDC3D7EF9F3F7F0606C3D7EF9F3F7F0808CEC9C3D7EF9F3F7F0707C9C3D7EF9F3F7F0808CCC9C3D7EF9F3F7F0404EF9F3F7F0808D2D5DBC7EF9F3F7F0707D5DBC7EF9F3F7F0808D0D5DBC7EF9F3F7F0606DBC7EF9F3F7F0808D6D1DBC7EF9F3F7F0707D1DBC7EF9F3F7F0808D4D1DBC7EF9F3F7F0505C7EF9F3F7F0808DADDD3C7EF9F3F7F0707DDD3C7EF9F3F7F0808D8DDD3C7EF9F3F7F0606D3C7EF9F3F7F0808DED9D3C7EF9F3F7F0707D9D3C7EF9F3F7F0808DCD9D3C7EF9F3F7F03039F3F7F0808E2E5EBF7CF9F3F7F0707E5EBF7CF9F3F7F0808E0E5EBF7CF9F3F7F0606EBF7CF9F3F7F0808E6E1EBF7CF9F3F7F0707E1EBF7CF9F3F7F0808E4E1EBF7CF9F3F7F0505F7CF9F3F7F0808EAEDE3F7CF9F3F7F0707EDE3F7CF9F3F7F0808E8EDE3F7CF9F3F7F0606E3F7CF9F3F7F0808EEE9E3F7CF9F3F7F0707E9E3F7CF9F3F7F0808ECE9E3F7CF9F3F7F0404CF9F3F7F0808F2F5FBE7CF9F3F7F0707F5FBE7CF9F3F7F0808F0F5FBE7CF9F3F7F0606FBE7CF9F3F7F0808F6F1FBE7CF9F3F7F0707F1FBE7CF9F3F7F0808F4F1FBE7CF9F3F7F0505E7CF9F3F7F0808FAFDF3E7CF9F3F7F0707FDF3E7CF9F3F7F0808F8FDF3E7CF9F3F7F0606F3E7CF9F3F7F0808FEF9F3E7CF9F3F7F0707F9F3E7CF9F3F7F";
    assert!(test_vector(
        copath_hex,
        0,
        size,
        FunctionType::TwoArgsPath(copath)
    ));
}
