//! # Integer Media File Parser
//!
//! This crate provides utilities for reading and parsing IMFs, a simple file format for storing 2D arrays.
use fancy_regex::Regex;
use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use colors_transform::{Color, Rgb};
#[derive(Clone)]
/// Stores the version, colors, width, height and content of an Integer Media File.
pub struct IMF {
    pub version: u8,
    pub colors: BTreeMap<i32, (u8, u8, u8)>,
    pub width: usize,
    pub height: usize,
    pub map: Vec<i32>,
}

impl IMF {
    /// Returns an IMF filled with default values.
    pub fn default() -> IMF {
        let mut m: BTreeMap<i32, (u8, u8, u8)> = BTreeMap::new();
        m.insert(0, (0, 0, 0));
        m.insert(1, (127, 127, 127));
        m.insert(2, (255, 255, 255));
        m.insert(3, (255, 0, 0));
        m.insert(4, (255, 127, 0));
        m.insert(5, (255, 255, 0));
        m.insert(6, (0, 255, 0));
        m.insert(7, (0, 0, 255));
        m.insert(8, (127, 0, 255));
        m.insert(9, (255, 0, 255));

        IMF {
            version: 1,
            colors: m,
            width: 8,
            height: 8,
            map: vec![1; 64],
        }
    }
    /// Creates new IMF from file located at filepath.
    /// If you want to create a new one from existing variables, declare it like this:
    /// ```
    /// use imf::IMF;
    ///
    /// //assuming that all variables already exist
    /// let imf = IMF {
    ///     version,
    ///     colors,
    ///     width,
    ///     height,
    ///     map
    /// };
    pub fn new(path: &str) -> Result<IMF, String> {
        let file_str = fs::read_to_string(path).map_err(|e| format!("Failed to read file '{path}': \n\t{e}"))?;
        let mut imf = IMF::default();

        let version = Self::proc_version(&file_str).map_err(|e| format!("IMF::Version: {e}"))?.unwrap_or_else(|| 1);
        imf.version = version;

        imf = match version {
            1 => Self::load_v1(imf.clone(), &file_str).map_err(|e| format!("IMF::LoadV1{e}"))?,
            2 => Self::load_v2(imf.clone(), &file_str).map_err(|e| format!("IMF::LoadV2{e}"))?,
            _ => return Err("Incompatible IMF version!".to_string())
        };

        Ok(imf)
    }

    fn load_v1(imf: IMF, file: &str) -> Result<IMF, String> {
        let mut i = imf;
        let mut lines = file.split('\n').filter(|line| !line.trim().is_empty());

        let width = lines.next().unwrap().parse().map_err(|_| "::Dimensions: Width not a number")?;
        let height = lines.next().unwrap().parse().map_err(|_| "::Dimensions: Height not a number")?;

        let mut map_str = String::new();

        while let Some(line) = lines.next() {
            map_str.push_str(line)
        }

        let map_arr = str2vec(map_str.as_str()).map_err(|e| format!("::Map: {e}"))?;

        let correct_size = width * height;

        if map_arr.len() != correct_size {
            let indic = if map_arr.len() > correct_size { "many" } else { "few" };
            return Err(format!("::Map: Too {indic} numbers in list"));
        }

        i.width = width;
        i.height = height;
        i.map = map_arr;

        Ok(i)
    }
    fn load_v2(imf: IMF, file: &str) -> Result<IMF, String> {
        let mut i = imf;

        let buffer = file.lines().fold(String::new(), |mut acc, line| {
            acc.push_str(line);
            acc
        });

        let clean_file = buffer.as_str();

        let col_map = Self::proc_cols(&clean_file).map_err(|e| format!("::Colors: {e}"))?;
        let (width, height) = Self::proc_dim(&clean_file).map_err(|e| format!("::Dimensions: {e}"))?;
        let map = Self::proc_map(&clean_file, width, height).map_err(|e| format!("::Map: {e}"))?;

        i.width = width;
        i.height = height;
        i.map = map;

        if col_map.is_some() { i.colors = col_map.unwrap() }

        Ok(i)
    }

    fn proc_version(file: &str) -> Result<Option<u8>, String> {
        let r = Regex::new(r"(?i)(?:\[v)(\d+)(?:])").unwrap();

        let version = match r.captures(file) {
            Ok(Some(m)) => m.get(1).unwrap().as_str(),
            Ok(None) => return Ok(None),
            Err(_) => return Err("Regex matching error".to_string())
        };

        Ok(version.parse().ok())
    }
    fn proc_dim(file: &str) -> Result<(usize, usize), String> {
        // matches with 'width/height'
        let r = Regex::new(r"\d+,\d+(?=\s*;)").unwrap();

        let dim_str = r.find(file).map_err(|_| "Regex matching error")?;
        if dim_str.is_none() { return Err("Dimensions not found".to_string()); }

        let dims: Vec<&str> = dim_str.unwrap().as_str().split(',').collect();
        if dims.len() != 2 { return Err("Invalid amount of dimensions".to_string()); }

        let x = dims[0].parse().map_err(|_| "Width is not a number")?;
        let y = dims[1].parse().map_err(|_| "Height is not a number")?;

        Ok((x, y))
    }
    fn proc_cols(file: &str) -> Result<Option<BTreeMap<i32, (u8, u8, u8)>>, String> {
        let r = Regex::new(r"(\d+\([0-9a-fA-F]{6}\))+").unwrap();
        let colors_str: &str;

        match r.find(file) {
            Ok(Some(c)) => colors_str = c.as_str(),
            Ok(None) => return Ok(None),
            Err(_) => return Err("Regex matching error".to_string())
        }

        let colors_list = colors_str.split(')').filter(|s| !s.is_empty()).collect::<Vec<&str>>();
        let mut color_map: BTreeMap<i32, (u8, u8, u8)> = BTreeMap::new();

        for c in colors_list {
            let key;
            let val;

            if let Some((key_str, col_str)) = c.split_once('(') {
                key = key_str.parse::<i32>().map_err(|_| format!("'{key_str}' not a number"))?;
                val = col_str;
            } else {
                return Err(format!("Incorrect formatting on line '{c})'"));
            }

            let hex = format!("#{val}");
            let rgb = Rgb::from_hex_str(hex.as_str()).map_err(|_| format!("'{hex}' is not a valid hex code!"))?;

            color_map.insert(key, (
                (rgb.get_red() * 255f32) as u8,
                (rgb.get_green() * 255f32) as u8,
                (rgb.get_blue() * 255f32) as u8),
            );
        }

        Ok(Some(color_map))
    }
    fn proc_map(file: &str, w: usize, h: usize) -> Result<Vec<i32>, String> {
        let r = Regex::new(r"(?<=\[)(\d+,?)+(?=\])").unwrap();

        let map_str: &str = match r.find(file).expect("Regex matching error") {
            Some(m) => m.as_str(),
            None => return Err("Integer list not found".to_string())
        };

        let map_arr = str2vec(map_str)?;

        if map_arr.len() != w * h {
            let indic = if map_arr.len() < w * h { "many" } else { "few" };
            return Err(format!("Too {indic} numbers in list"));
        }

        Ok(map_arr.to_vec())
    }

    ///Returns number found at coordinates within IMF.
    ///See [`IMF::set_xy`]
    /// ## Arguments
    /// * `x` - The X coordinate
    /// * `y` - The Y coordinate
    ///
    ///## Example
    ///```
    /// use imf::IMF;
    /// //example.imf:
    /// //1,0,1,5,
    /// //4,7,3,3,
    /// //9,2,5,6,
    /// //0,5,8,2
    ///
    /// let mut imf = IMF::new("example.imf").unwrap();
    /// let n = imf.get_xy(1,1).unwrap();
    ///
    /// // n == 7
    pub fn get_xy(&self, x: usize, y: usize) -> Option<i32> {
        let index = self.xy2i(x, y);
        self.map.get(index).ok()
    }
    ///Sets number at coordinates within IMF to the number specified.
    ///See [`IMF::get_xy`]
    /// ## Arguments
    /// * `x` - The X coordinate
    /// * `y` - The Y coordinate
    /// * `i` - What the number will be set to
    ///## Example
    ///```
    /// use imf::IMF;
    ///
    /// let mut imf = IMF::new("example.imf").unwrap();
    /// imf.set_xy(2,2,5).expect("Coordinates out of range!");
    ///
    /// // imf.get_xy(2,2).unwrap() == 5
    pub fn set_xy(&mut self, x: usize, y: usize, i: i32) -> Result<(), String> {
        let index = self.xy2i(x, y);
        let val = self.map.get_mut(index.ok_or("Coordinates out of range!".to_string())?).unwrap();
        *val = i;
        Ok(())
    }
    /// Converts XY coordinates to an index.
    /// See [`IMF::i2xy`]
    /// ## Example
    /// ```
    /// use imf::IMF;
    /// //example.imf:
    /// //1,0,1,5,
    /// //4,7,3,3,
    /// //9,2,5,6,
    /// //0,5,8,2
    ///
    /// let imf = IMF::new("example.imf").unwrap();
    /// let n = imf.xy2i(2,2);
    ///
    /// // n == 10
    pub fn xy2i(&self, x: usize, y: usize) -> Option<usize> {
        if x > self.width || y > self.height {
            return None;
        }

        Some(y * self.width + x)
    }
    /// Converts XY coordinates to an index.
    /// See [`IMF::xy2i`]
    /// ## Example
    /// ```
    /// use imf::IMF;
    /// //example.imf:
    /// //1,0,1,5,
    /// //4,7,3,3,
    /// //9,2,5,6,
    /// //0,5,8,2
    ///
    /// let imf = IMF::new("example.imf").unwrap();
    /// let n = imf.xy2i(2,2);
    /// let m = imf.i2xy(10);
    ///
    /// // n == m
    pub fn i2xy(&self, i: usize) -> Option<(usize, usize)> {
        if i < self.map.len() {
            let y = i / self.width;
            let x = i % self.width;
            Some((x, y))
        } else {
            None
        }
    }

    ///Writes IMF to given filepath in .imf form
    pub fn write(&self, path: &str) -> Result<(), String> {
        let mut file = File::create(path).map_err(|e| e.to_string())?;

        match self.version {
            1 => {
                writeln!(file, "{}", self.width).map_err(|e| e.to_string())?;
                writeln!(file, "{}", self.height).map_err(|e| e.to_string())?;

                for y in 0..self.height {
                    for x in 0..self.width {
                        let index = self.xy2i(x, y).unwrap();
                        write!(file, "{},", self.map.get(index).unwrap()).map_err(|e| e.to_string())?;
                    }
                    writeln!(file).map_err(|e| e.to_string())?;
                }
            }
            2 => {
                writeln!(file, "[v2]").map_err(|e| e.to_string())?;
                writeln!(file, "{},{};", self.width, self.height).map_err(|e| e.to_string())?;
                for col in self.colors.clone() {
                    let (index, (r, g, b)) = col;
                    let color = Rgb::from_tuple(&(r as f32, g as f32, b as f32)).to_css_hex_string()
                        .replace('#', "");

                    writeln!(file, "{index}({color})").map_err(|e| e.to_string())?;
                }
                writeln!(file, "[").map_err(|e| e.to_string())?;
                for y in 0..self.height {
                    for x in 0..self.width {
                        let index = self.xy2i(x, y).unwrap();
                        write!(file, "{},", self.map.get(index).unwrap()).map_err(|e| e.to_string())?;
                    }
                    writeln!(file).map_err(|e| e.to_string())?;
                }
                writeln!(file, "]").map_err(|e| e.to_string())?;
            }
            _ => {}
        }

        Ok(())
    }
}

/// Converts string to vector of integers
/// ## Example
/// ```
/// use imf::str2vec;
///
/// //works with all spacings
/// let vec = str2vec("0,1, 2, 3 ,4 ,5");
///
/// // vec == vec![0,1,2,3,4,5];
pub fn str2vec(str: &str) -> Result<Vec<i32>, String> {
    let mut map = Vec::new();

    for item in str.split(',') {
        let t = item.trim();

        if t.is_empty() { continue; };

        match t.parse::<i32>() {
            Ok(n) => map.push(n),
            Err(_) => return Err(format!("'{t}' is not a number!"))
        }
    }

    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test() {
    //     let i = IMF::new("export2.imf").unwrap();
    //     i.write("export2.imf").map_err(|e| println!("ERROR: {}", e)).ok();
    // }
}