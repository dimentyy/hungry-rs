use indexmap::{IndexMap, IndexSet};

use crate::meta::temp::{TempEnum, TempFunc, TempType};
use crate::meta::{
    Arg, ArgTyp, Combinator, Deserialization, Enum, Flag, Func, GenericArg, Ident, Temp, Typ, Type,
};
use crate::{read, rust};

#[derive(Debug)]
pub(crate) struct Data<'a> {
    pub(crate) types: Vec<Type<'a>>,
    pub(crate) funcs: Vec<Func<'a>>,
    pub(crate) enums: Vec<Enum<'a>>,

    pub(crate) types_split: Vec<usize>,
    pub(crate) funcs_split: Vec<usize>,
    pub(crate) enums_split: Vec<usize>,
}

impl<'a> Data<'a> {
    pub(super) fn validate(temp: Temp<'a>) -> Self {
        let mut data = Self {
            types: Vec::with_capacity(temp.types.len()),
            funcs: Vec::with_capacity(temp.funcs.len()),
            enums: Vec::with_capacity(temp.enums.len()),

            types_split: Vec::new(),
            funcs_split: Vec::new(),
            enums_split: Vec::new(),
        };

        for x in temp.types.values() {
            data.push_type(&temp, x);
        }

        for (ident, x) in &temp.enums {
            data.push_enum(&temp, ident, x);
        }

        for x in temp.funcs.values() {
            data.push_func(&temp, x);
        }

        let mut visited_types = Vec::with_capacity(data.types.len());
        visited_types.resize(visited_types.capacity(), None);

        for i in 0..data.types.len() {
            data.types[i].recursive = data.check_recursion(&mut visited_types, i);
        }

        let mut visited_types = Vec::with_capacity(data.types.len());
        visited_types.resize(visited_types.capacity(), false);

        let mut visited_enums = Vec::with_capacity(data.enums.len());
        visited_enums.resize(visited_enums.capacity(), false);

        for i in 0..data.types.len() {
            data.types[i].combinator.de = data.type_de(i, &mut visited_types, &mut visited_enums);
            visited_types.fill(false);
            visited_enums.fill(false);
        }

        for i in 0..data.enums.len() {
            data.enums[i].de = data.enum_de(i, &mut visited_types, &mut visited_enums);
            visited_types.fill(false);
            visited_enums.fill(false);
        }

        for i in 0..data.funcs.len() {
            data.funcs[i].combinator.de = data.func_de(i);
            visited_types.fill(false);
            visited_enums.fill(false);
        }

        data.types_split = temp.types_split;
        data.funcs_split = temp.funcs_split;
        data.enums_split = temp.enums_split;

        data.types_split.push(data.types.len());
        data.funcs_split.push(data.funcs.len());
        data.enums_split.push(data.enums.len());

        dbg!(data)
    }

    fn func_de(&self, i: usize) -> Deserialization {
        let mut infallible = true;
        let mut len = 0;

        for arg in &self.funcs[i].combinator.args {
            let typ = match &arg.typ {
                ArgTyp::Flags { .. } => {
                    len += 4;
                    continue;
                }
                ArgTyp::Typ { typ, flag, .. } => {
                    if flag.is_some() {
                        return Deserialization::Checked;
                    }

                    typ
                }
                ArgTyp::True { .. } => continue,
            };

            match typ.ready_de(self) {
                Deserialization::Infallible(n) => len += n,
                Deserialization::Unchecked(n) => {
                    infallible = false;
                    len += n;
                }
                Deserialization::Checked => return Deserialization::Checked,
            };
        }

        if infallible {
            Deserialization::Infallible(len)
        } else {
            Deserialization::Unchecked(len)
        }
    }

    pub(crate) fn type_de(
        &self,
        i: usize,
        visited_types: &mut Vec<bool>,
        visited_enums: &mut Vec<bool>,
    ) -> Deserialization {
        if visited_types[i] {
            return self.types[i].combinator.de;
        }

        visited_types[i] = true;

        let mut infallible = true;
        let mut len = 0;

        for arg in &self.types[i].combinator.args {
            let typ = match &arg.typ {
                ArgTyp::Flags { .. } => {
                    len += 4;
                    continue;
                }
                ArgTyp::Typ { typ, flag, .. } => {
                    if flag.is_some() {
                        return Deserialization::Checked;
                    }

                    typ
                }
                ArgTyp::True { .. } => continue,
            };

            match typ.de(self, visited_types, visited_enums) {
                Deserialization::Infallible(n) => len += n,
                Deserialization::Unchecked(n) => {
                    infallible = false;
                    len += n;
                }
                Deserialization::Checked => return Deserialization::Checked,
            };
        }

        if infallible {
            Deserialization::Infallible(len)
        } else {
            Deserialization::Unchecked(len)
        }
    }

    pub(crate) fn enum_de(
        &self,
        i: usize,
        visited_types: &mut Vec<bool>,
        visited_enums: &mut Vec<bool>,
    ) -> Deserialization {
        visited_enums[i] = true;

        let variants = &self.enums[i].variants;

        let de = self.type_de(variants[0], visited_types, visited_enums);
        for &x in &variants[1..] {
            if self.type_de(x, visited_types, visited_enums) != de {
                return Deserialization::Checked;
            }
        }

        match de {
            Deserialization::Infallible(len) => Deserialization::Unchecked(len + 4),
            Deserialization::Unchecked(len) => Deserialization::Unchecked(len + 4),
            x => x,
        }
    }

    fn push_type(&mut self, temp: &Temp<'a>, x: &TempType<'a>) {
        if !x.combinator.opts.is_empty() {
            todo!()
        }

        let (combinator, _) = Self::combinator(temp, x.combinator);

        self.types.push(Type {
            combinator,
            enum_index: x.enum_index,
            recursive: false,
        });
    }

    fn push_func(&mut self, temp: &Temp<'a>, x: &TempFunc<'a>) {
        let (combinator, generic_args) = Self::combinator(temp, x.combinator);
        let response = Self::typ(temp, &x.combinator.result, &generic_args);

        self.funcs.push(Func {
            combinator,
            response,
        });
    }

    fn push_enum(&mut self, _temp: &Temp, ident: &'a read::Ident<'a>, x: &TempEnum) {
        self.enums.push(Enum {
            parsed: ident,
            ident: Ident::from(ident),
            variants: x.bare_types.clone(),
            de: Deserialization::Checked,
        })
    }

    pub(super) fn combinator(
        temp: &Temp<'a>,
        combinator: &'a read::Combinator<'a>,
    ) -> (Combinator<'a>, IndexSet<&'a str>) {
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

                    if typ.ident == read::Ident::TRUE {
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
                parsed: combinator,
                ident: Ident::from(&combinator.ident),
                explicit_id: combinator.name,
                inferred_id: combinator.infer_name(),
                args: args.into_values().collect(),
                generic_args: generic_args
                    .iter()
                    .map(|s| GenericArg {
                        ident: rust::pascal_case(*s),
                    })
                    .collect(),
                de: Deserialization::Checked,
            },
            generic_args,
        )
    }

    pub(super) fn typ(temp: &Temp, typ: &read::Typ, generic_args: &IndexSet<&str>) -> Typ {
        macro_rules! ok {
            ($len:expr ; $typ:expr) => {{
                if typ.params.len() != $len {
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
            ok!(0; Typ::Type { index });
        }

        if let Some(index) = temp.enums.get_index_of(&typ.ident) {
            ok!(0; Typ::Enum { index })
        }

        todo!()
    }

    pub(super) fn check_recursion(
        &self,
        visited: &mut Vec<Option<Option<bool>>>,
        current: usize,
    ) -> bool {
        match visited[current] {
            Some(Some(recursive)) => return recursive,
            Some(None) => return true,
            None => {}
        }

        visited[current] = Some(None);

        for arg in &self.types[current].combinator.args {
            let typ = match &arg.typ {
                ArgTyp::Flags { .. } => continue,
                ArgTyp::Typ { typ, .. } => typ,
                ArgTyp::True { .. } => continue,
            };

            if typ.check_recursion(self, visited) {
                visited[current] = Some(Some(true));
                return true;
            }
        }

        visited[current] = Some(Some(false));

        false
    }
}
