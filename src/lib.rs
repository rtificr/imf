use bytebuilder::{builder::ByteBuilder, reader::ByteReader, traits::BytesTrait};

pub type TileType = i16;

#[derive(Clone, PartialEq, Eq)]
pub struct IMF {
    pub width: u32,
    pub height: u32,
    pub layers: Vec<Vec<Tile>>,
}
impl std::fmt::Debug for IMF {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "IMF {{")?;
        writeln!(f, "    width: {},", self.width)?;
        writeln!(f, "    height: {},", self.height)?;
        writeln!(f, "    map: [")?;
        for map in self.layers.iter() {
            for chunk in map.chunks(self.width as usize) {
                writeln!(f, "        {:?},", chunk)?;
            }
        }

        writeln!(f, "    ]")?;
        writeln!(f, "}}")
    }
}
impl IMF {
    pub fn new(width: u32, height: u32, fill: Tile) -> IMF {
        IMF {
            width,
            height,
            layers: vec![vec![fill; (width * height) as usize]],
        }
    }
    pub fn new_with_layers(width: u32, height: u32, fill: Vec<Tile>) -> Result<IMF, ()> {
        Ok(IMF {
            width,
            height,
            layers: fill.iter().map(|f| vec![f.clone(); (width * height) as usize]).collect(),
        })
    }

    pub(crate) fn ser_v3(&self) -> Vec<u8> {
        let mut bb = ByteBuilder::new();
        bb.push_u8(3);
        bb.push_u32(self.width);
        bb.push_u32(self.height);
        bb.push_u32(self.layers.len() as u32);
        for map in &self.layers {
            for tile in map {
                match tile {
                    Tile::Int(t) => {
                        bb.push_u8(0);
                        bb.push_i16(*t)
                    }
                    Tile::Sides(sides) => {
                        bb.push_u8(1);
                        bb.push_i16(sides.n);
                        bb.push_i16(sides.e);
                        bb.push_i16(sides.s);
                        bb.push_i16(sides.w);
                    }
                }
            }
        }
        bb.bytes
    }

    pub(crate) fn deser_v3(br: &mut ByteReader) -> Option<Self> {
        let width = br.read_u32()?;
        let height = br.read_u32()?;
        let layer_count = br.read_u32()?;
        let mut layers = Vec::new();
        for _ in 0..layer_count {
            let mut layer = Vec::new();
            for _ in 0..(width * height) {
                let tile_type = br.read_u8()?;
                match tile_type {
                    0 => {
                        let t = br.read_i16()?;
                        layer.push(Tile::Int(t));
                    }
                    1 => {
                        let n = br.read_i16()?;
                        let e = br.read_i16()?;
                        let s = br.read_i16()?;
                        let w = br.read_i16()?;
                        layer.push(Tile::Sides(Sides { n, e, s, w }));
                    }
                    _ => return None,
                }
            }
            layers.push(layer);
        }
        Some(IMF { width, height, layers })
    }
}

impl BytesTrait for IMF {
    fn to_bytes(&self) -> Vec<u8> {
        self.ser_v3()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut br = ByteReader::new(bytes);
        let version = br.read_u8()?;
        match version {
            3 => IMF::deser_v3(&mut br),
            _ => None,
        }
    }
}

impl Default for IMF {
    fn default() -> Self {
        IMF::new(8, 8, Tile::Int(0))
    }
}
#[derive(Clone, PartialEq, Eq)]
pub enum Tile {
    Int(TileType),
    Sides(Sides),
}
impl std::fmt::Debug for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tile::Int(t) => write!(f, "i{}", t),
            Tile::Sides(sides) => write!(f, "s[{:?}]", sides),
        }
    }
}

impl Tile {
    pub fn is_int(&self) -> bool {
        matches!(self, Tile::Int(_))
    }
    pub fn is_sides(&self) -> bool {
        matches!(self, Tile::Sides(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sides {
    pub n: TileType,
    pub e: TileType,
    pub s: TileType,
    pub w: TileType,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_imf() {
        let imf = IMF::new_with_layers(3, 3, vec![Tile::Int(0), Tile::Int(1), Tile::Int(2)]).unwrap();
        println!("{:?}", imf);
        let bytes = imf.to_bytes();
        let imf2 = IMF::from_bytes(&bytes).unwrap();
        assert_eq!(imf.width, imf2.width);
        assert_eq!(imf.height, imf2.height);
        assert_eq!(imf.layers, imf2.layers);
    }
}
