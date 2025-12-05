use chumsky::container::Container;
use indexmap::{IndexMap, IndexSet};
use std::collections::HashSet;
use std::mem;

use crate::meta::{Arg, ArgTyp, Combinator, Error, Flag, GenericArg, Name, Temp, Typ, TypeOrEnum};
use crate::{read, rust};

#[derive(Debug)]
pub struct Type {
    pub combinator: Combinator,
    pub enum_index: usize,
    pub recursive: bool,
}

#[derive(Debug)]
pub struct Func {
    pub combinator: Combinator,
    pub response: Typ,
}

#[derive(Debug)]
pub struct Enum {
    pub name: Name,
    pub variants: Vec<usize>,
}

#[derive(Debug)]
pub(crate) struct Data {
    pub(crate) types: Vec<Type>,
    pub(crate) funcs: Vec<Func>,
    pub(crate) enums: Vec<Enum>,
}

impl Data {
    pub(super) fn validate(temp: Temp<'_>) -> Result<Self, Error> {
        let mut data = Self {
            types: Vec::with_capacity(temp.types.len()),
            funcs: Vec::with_capacity(temp.funcs.len()),
            enums: Vec::with_capacity(temp.enums.len()),
        };

        for (combinator, enum_index) in temp.types.values() {
            data.push_type(&temp, combinator, *enum_index)?;
        }

        for x in temp.funcs.values() {
            data.push_func(&temp, x)?;
        }

        for (ident, variants) in &temp.enums {
            data.push_enum(&temp, ident, variants)?;
        }

        for i in 0..data.types.len() {
            data.types[i].recursive = data.check_recursion(
                &mut HashSet::new(),
                TypeOrEnum::Type(i),
                TypeOrEnum::Type(i),
            );
        }

        Ok(data)
    }

    pub(super) fn push_type(
        &mut self,
        temp: &Temp,
        combinator: &read::Combinator,
        enum_index: usize,
    ) -> Result<(), Error> {
        let (combinator, _) = Self::combinator(temp, combinator)?;

        if !combinator.generic_args.is_empty() {
            unimplemented!()
        }

        self.types.push(Type {
            combinator,
            enum_index,
            recursive: false,
        });

        Ok(())
    }

    pub(super) fn push_func(
        &mut self,
        temp: &Temp,
        parsed_combinator: &read::Combinator,
    ) -> Result<(), Error> {
        let (combinator, generic_args) = Self::combinator(temp, parsed_combinator)?;
        let response = Self::typ(temp, &parsed_combinator.result, &generic_args)?;

        self.funcs.push(Func {
            combinator,
            response,
        });

        Ok(())
    }

    pub(super) fn push_enum(
        &mut self,
        temp: &Temp,
        ident: &read::Ident,
        variants: &Vec<usize>,
    ) -> Result<(), Error> {
        self.enums.push(Enum {
            name: Name::from(ident),
            variants: variants.clone(),
        });

        Ok(())
    }

    pub(super) fn typ(
        temp: &Temp,
        typ: &read::Typ,
        generic_args: &IndexSet<&str>,
    ) -> Result<Typ, Error> {
        fn check_params(params: &Vec<read::Typ>, typ: Typ) -> Result<Typ, Error> {
            if !params.is_empty() {
                unimplemented!()
            }

            Ok(typ)
        }

        match typ.ident {
            read::Ident { name, space: None } => match name {
                "true" => unimplemented!(),
                "int" => return check_params(&typ.params, Typ::Int),
                "long" => return check_params(&typ.params, Typ::Long),
                "double" => return check_params(&typ.params, Typ::Double),
                "bytes" => return check_params(&typ.params, Typ::Bytes),
                "string" => return check_params(&typ.params, Typ::String),
                "Bool" => return check_params(&typ.params, Typ::Bool),
                "vector" => {
                    if typ.params.len() != 1 {
                        unimplemented!()
                    }

                    return Ok(Typ::BareVector(Box::new(Self::typ(
                        temp,
                        &typ.params[0],
                        generic_args,
                    )?)));
                }
                "Vector" => {
                    if typ.params.len() != 1 {
                        unimplemented!()
                    }

                    return Ok(Typ::Vector(Box::new(Self::typ(
                        temp,
                        &typ.params[0],
                        generic_args,
                    )?)));
                }
                "int128" => return check_params(&typ.params, Typ::Int128),
                "int256" => return check_params(&typ.params, Typ::Int256),
                name => {
                    if let Some(index) = generic_args.get_index_of(name) {
                        return check_params(&typ.params, Typ::Generic { index });
                    }
                }
            },
            _ => {}
        };

        let mut params = Vec::with_capacity(typ.params.len());

        for param in &typ.params {
            params.push(Self::typ(temp, param, generic_args)?);
        }

        if let Some(index) = temp.types.get_index_of(&typ.ident) {
            return Ok(Typ::Type { index, params });
        }

        if let Some(index) = temp.enums.get_index_of(&typ.ident) {
            return Ok(Typ::Enum { index, params });
        }

        unimplemented!()
    }

    pub(super) fn combinator<'a>(
        temp: &Temp<'a>,
        combinator: &read::Combinator<'a>,
    ) -> Result<(Combinator, IndexSet<&'a str>), Error> {
        let mut generic_args =
            IndexSet::with_capacity(combinator.opts.iter().map(|o| o.idents.len()).sum());

        for opt in &combinator.opts {
            match opt.typ {
                read::OptArgsTyp::Type => {
                    for ident in &opt.idents {
                        if !generic_args.insert(*ident) {
                            unimplemented!()
                        }
                    }
                }
            }
        }

        let mut args = IndexMap::with_capacity(combinator.args.len());

        for (i, arg) in combinator.args.iter().enumerate() {
            let typ = match &arg.typ {
                read::ArgTyp::Typ {
                    excl_mark,
                    typ,
                    flag,
                } => {
                    let flag = match flag {
                        None => None,
                        Some(flag) => Some(Flag::find(&mut args, i, flag)?),
                    };

                    if typ.ident == read::Ident::TRUE {
                        let Some(flag) = flag else {
                            unimplemented!();
                        };

                        if *excl_mark {
                            unimplemented!();
                        }

                        ArgTyp::True { flag }
                    } else {
                        let typ = Self::typ(temp, typ, &generic_args)?;

                        ArgTyp::Typ { typ, flag }
                    }
                }
                read::ArgTyp::Nat => ArgTyp::Flags { args: Vec::new() },
            };

            let name = rust::snake_case(arg.ident);

            if let Some(_) = args.insert(arg.ident, Arg { name, typ }) {
                unimplemented!()
            }
        }

        Ok((
            Combinator {
                name: Name::from(&combinator.ident),
                explicit_id: combinator.name,
                inferred_id: combinator.infer_name(),
                args: args.into_values().collect(),
                generic_args: generic_args
                    .iter()
                    .map(|s| GenericArg {
                        name: rust::pascal_case(s),
                    })
                    .collect(),
            },
            generic_args,
        ))
    }

    pub(crate) fn check_recursion(
        &self,
        visited: &mut HashSet<TypeOrEnum>,
        root: TypeOrEnum,
        value: TypeOrEnum,
    ) -> bool {
        if visited.contains(&value) {
            return !visited.is_empty();
        }

        visited.push(value);

        match value {
            TypeOrEnum::Type(index) => {
                for arg in &self.types[index].combinator.args {
                    let typ = match &arg.typ {
                        ArgTyp::Flags { .. } => continue,
                        ArgTyp::Typ { typ, .. } => typ,
                        ArgTyp::True { .. } => continue,
                    };

                    if typ.check_recursion(self, visited, root) {
                        return true;
                    }
                }
            }
            TypeOrEnum::Enum(index) => {
                for x in &self.enums[index].variants {
                    if self.check_recursion(visited, root, TypeOrEnum::Type(*x)) {
                        return true;
                    }
                }
            }
        };

        false
    }
}
