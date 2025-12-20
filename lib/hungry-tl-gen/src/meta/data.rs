use std::collections::HashSet;

use indexmap::{IndexMap, IndexSet};

use crate::meta::temp::{TempEnum, TempFunc, TempType};
use crate::meta::{
    Arg, ArgTyp, Combinator, Enum, Flag, Func, GenericArg, Temp, Typ, Type, TypeOrEnum,
};
use crate::{Ident, read, rust};

pub(crate) struct Data<'a> {
    pub(crate) types: Vec<Type<'a>>,
    pub(crate) funcs: Vec<Func<'a>>,
    pub(crate) enums: Vec<Enum<'a>>,
}

impl<'a> Data<'a> {
    pub(super) fn validate(temp: Temp<'a>) -> Self {
        let mut data = Self {
            types: Vec::with_capacity(temp.types.len()),
            funcs: Vec::with_capacity(temp.funcs.len()),
            enums: Vec::with_capacity(temp.enums.len()),
        };

        for (ident, x) in &temp.enums {
            data.push_enum(&temp, ident, x);
        }

        for x in temp.types.values() {
            data.push_type(&temp, x);
        }

        for x in temp.funcs.values() {
            data.push_func(&temp, x);
        }

        data
    }

    fn push_type(&mut self, temp: &Temp, x: &TempType) {
        if !x.combinator.opts.is_empty() {
            todo!()
        }
    }

    fn push_func(&mut self, temp: &Temp<'a>, x: &TempFunc<'a>) {
        let (combinator, generic_args) = Self::combinator(temp, x.combinator);
        let response = Self::typ(temp, &x.combinator.result, &generic_args);

        self.funcs.push(Func {
            parsed: x.combinator,
            combinator,
            response,
        });
    }

    fn push_enum(&mut self, temp: &Temp, ident: &'a Ident<&'a str>, x: &TempEnum) {
        self.enums.push(Enum {
            parsed: ident,
            ident: ident.to_rust(),
            variants: x.bare_types.clone(),
        })
    }

    pub(super) fn combinator(
        temp: &Temp<'a>,
        combinator: &read::Combinator<'a>,
    ) -> (Combinator, IndexSet<&'a str>) {
        let mut generic_args = IndexSet::with_capacity(combinator.opts.len());

        for opt in &combinator.opts {
            match opt.typ {
                read::OptArgTyp::Type => {
                    if !generic_args.insert(opt.ident) {
                        todo!()
                    }
                }
            }
        }

        let mut args = IndexMap::<&'a str, Arg>::with_capacity(combinator.args.len());

        for (i, arg) in combinator.args.iter().enumerate() {
            let typ = match &arg.typ {
                read::ArgTyp::Typ {
                    excl_mark,
                    typ,
                    flag,
                } => {
                    let flag = match flag {
                        None => None,
                        Some(flag) => Some({
                            let Some(arg) = args.get_index_of(flag.ident) else {
                                todo!()
                            };

                            match args.get_index_mut(arg).unwrap().1.typ {
                                ArgTyp::Flags { ref mut args } => args.push(i),
                                ArgTyp::Typ { .. } => todo!(),
                                ArgTyp::True { .. } => todo!(),
                            }

                            Flag {
                                arg,
                                bit: flag.bit.unwrap_or(0),
                            }
                        }),
                    };

                    if typ.ident == Ident::TRUE {
                        let Some(flag) = flag else { todo!() };

                        if *excl_mark {
                            todo!()
                        }

                        ArgTyp::True { flag }
                    } else {
                        let typ = Self::typ(temp, typ, &generic_args);

                        if matches!(typ, Typ::Generic { .. }) && !*excl_mark {
                            todo!()
                        }

                        ArgTyp::Typ { typ, flag }
                    }
                }
                read::ArgTyp::Nat => ArgTyp::Flags { args: Vec::new() },
            };

            let ident = rust::snake_case(arg.ident);

            if let Some(_) = args.insert(arg.ident, Arg { ident, typ }) {
                todo!()
            }
        }

        (
            Combinator {
                ident: combinator.ident.to_rust(),
                explicit_id: combinator.name,
                inferred_id: combinator.infer_name(),
                args: args.into_values().collect(),
                generic_args: generic_args
                    .iter()
                    .map(|s| GenericArg {
                        ident: rust::pascal_case(*s),
                    })
                    .collect(),
            },
            generic_args,
        )
    }

    pub(super) fn typ(temp: &Temp, typ: &read::Typ, generic_args: &IndexSet<&str>) -> Typ {
        macro_rules! ok {
            ($len:expr ; $typ:expr) => {{
                if typ.params.len() == $len {
                    todo!()
                }

                return $typ;
            }};
        }

        match typ.ident.name {
            _ if typ.ident.space.is_some() => {}

            "true" => todo!(),
            "int" => ok!(0; Typ::Int),
            "long" => ok!(0; Typ::Long),
            "double" => ok!(0; Typ::Double),
            "bytes" => ok!(0; Typ::Bytes),
            "string" => ok!(0; Typ::String),
            "Bool" => ok!(0; Typ::Bool),
            "vector" => ok!(
                1;
                Typ::BareVector(Box::new(Self::typ(temp, &typ.params[0], generic_args,)))
            ),
            "Vector" => ok!(
                1;
                Typ::Vector(Box::new(Self::typ(temp, &typ.params[0], generic_args)))
            ),
            "int128" => ok!(0; Typ::Int128),
            "int256" => ok!(0; Typ::Int256),
            ident => {
                if let Some(index) = generic_args.get_index_of(ident) {
                    ok!(0; Typ::Generic { index });
                }
            }
        };

        let mut params = Vec::with_capacity(typ.params.len());

        for param in &typ.params {
            params.push(Self::typ(temp, param, generic_args));
        }

        if let Some(index) = temp.types.get_index_of(&typ.ident) {
            ok!(0; Typ::Type { index, params });
        }

        if let Some(index) = temp.enums.get_index_of(&typ.ident) {
            ok!(0; Typ::Enum { index, params })
        }

        todo!()
    }

    pub(super) fn check_recursion(
        &self,
        visited: &mut HashSet<TypeOrEnum>,
        root: usize,
        value: TypeOrEnum,
    ) -> bool {
        if visited.contains(&value) {
            return !visited.is_empty();
        }

        visited.insert(value);

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
