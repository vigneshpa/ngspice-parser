use serde::Serialize;
#[derive(Debug, Clone, Copy, Serialize)]
pub enum Flags {
    Complex,
    Real,
}
#[derive(Debug, Serialize)]
pub struct VarData {
    pub name: String,
    pub typee: String,
    pub values: Vec<f64>,
    pub angles: Option<Vec<f64>>,
}
#[derive(Debug, Serialize)]
pub struct Plot {
    pub title: String,
    pub date: String,
    pub plotname: String,
    pub flags: Flags,
    pub no_of_variables: usize,
    pub no_of_points: usize,
    pub data: Vec<VarData>,
}

#[derive(thiserror::Error, Debug)]
pub enum SpiceParseError {
    #[error("Cannot parse integer")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("Cannot parse float")]
    ParseFloat(#[from] std::num::ParseFloatError),
    #[error("Number of variables mismatch")]
    NoOfVarMismatch,
    #[error("Number of values mismatch")]
    NoOfValMismatch,
    #[error("Unknown value in flags")]
    UnknownFlag,
}
fn flush_values(
    no_of_variables: usize,
    temp_values: &mut Vec<(f64, f64)>,
    data: &mut Vec<VarData>,
    flags: Flags,
) -> Result<(), SpiceParseError> {
    if temp_values.len() != 0 {
        if temp_values.len() != no_of_variables {
            return Result::Err(SpiceParseError::NoOfValMismatch);
        }
        let mut idx: usize = 0;
        for val in temp_values.iter() {
            data[idx].values.push(val.0);
            if let Flags::Complex = flags {
                if let Option::Some(vec) = &mut data[idx].angles {
                    vec.push(val.1);
                }
            }
            idx += 1;
        }
        temp_values.clear();
    }
    Ok(())
}
pub fn parse(file: &str) -> Result<Plot, SpiceParseError> {
    let mut title: String = String::new();
    let mut date: String = String::new();
    let mut plotname: String = String::new();
    let mut flags: Flags = Flags::Real;
    let mut no_of_variables: usize = 0;
    let mut no_of_points: usize = 0;
    let mut data: Vec<VarData> = Vec::new();
    enum Modes {
        Meta,
        Variable,
        Value,
    }
    let mut mode: Modes = Modes::Meta;
    let mut variable_counter: usize = 0;
    let mut temp_values: Vec<(f64, f64)> = Vec::new();
    for lin in file.lines() {
        if lin.trim().len() == 0 {
            continue;
        }
        match mode {
            Modes::Meta => {
                let parts: Vec<&str> = lin.trim().split(':').collect();
                match parts[0] {
                    "Title" => title = String::from(parts[1].trim()),
                    "Date" => date = String::from(parts[1..].join("").trim()),
                    "Plotname" => plotname = String::from(parts[1].trim()),
                    "Flags" => {
                        flags = match parts[1].trim() {
                            "complex" => Flags::Complex,
                            "real" => Flags::Real,
                            _ => {
                                return Result::Err(SpiceParseError::UnknownFlag);
                            }
                        }
                    }
                    "No. Variables" => no_of_variables = parts[1].trim().parse()?,
                    "No. Points" => no_of_points = parts[1].trim().parse()?,
                    "Variables" => mode = Modes::Variable,
                    "Values" => mode = Modes::Value,
                    _ => {}
                };
            }
            Modes::Variable => {
                if variable_counter == no_of_variables {
                    return Result::Err(SpiceParseError::NoOfVarMismatch);
                }
                variable_counter += 1;

                if variable_counter == no_of_variables {
                    mode = Modes::Meta;
                }
                let parts: Vec<&str> = lin.trim().split("\t").collect();
                data.push(VarData {
                    name: String::from(parts[1].trim()),
                    typee: String::from(parts[2].trim()),
                    values: Vec::new(),
                    angles: match flags {
                        Flags::Real => None,
                        Flags::Complex => Some(Vec::new()),
                    },
                })
            }
            Modes::Value => {
                let parts: Vec<&str> = lin.trim().split('\t').collect();
                let mut num = parts[0];
                if parts.len() == 2 {
                    flush_values(no_of_variables, &mut temp_values, &mut data, flags)?;
                    num = parts[1];
                };
                let val = match flags {
                    Flags::Real => (num.parse()?, 0f64),
                    Flags::Complex => {
                        let pts: Vec<&str> = num.split(",").collect();
                        let real: f64 = pts[0].parse()?;
                        let imaginary: f64 = pts[1].parse()?;
                        (
                            (real.powi(2) + imaginary.powi(2)).sqrt(),
                            (imaginary / real).atan(),
                        )
                    }
                };
                temp_values.push(val);
            }
        };
    }
    flush_values(no_of_variables, &mut temp_values, &mut data, flags)?;
    Result::Ok(Plot {
        title,
        date,
        plotname,
        flags,
        no_of_variables,
        no_of_points,
        data,
    })
}
pub fn parse_and_get_csv(file: &str) -> Result<String, SpiceParseError> {
    let mut ret = String::new();
    let plot = parse(file)?;
    for var_data in plot.data.iter() {
        ret += var_data.name.as_str();
        ret += " - ";
        ret += var_data.typee.as_str();
        ret += ",";
        if let Flags::Complex = plot.flags {
            ret += var_data.typee.as_str();
            ret += "(phase),";
        }
    }
    ret.remove(ret.len() - 1);
    ret += "\n";
    for i in 0..plot.no_of_points {
        for j in 0..plot.no_of_variables {
            let val: String = match plot.flags {
                Flags::Real => plot.data[j].values[i].to_string(),
                Flags::Complex => {
                    if let Some(angles) = &plot.data[j].angles {
                        let mut a = plot.data[j].values[i].to_string();
                        a += ",";
                        a += angles[i].to_degrees().to_string().as_str();
                        a += "°";
                        a
                    } else {
                        String::from("")
                    }
                }
            };
            ret += val.as_str();
            ret += if j != (plot.no_of_variables - 1) {
                ","
            } else {
                "\n"
            };
        }
    }
    Ok(ret)
}
#[cfg(test)]
pub mod tests;
