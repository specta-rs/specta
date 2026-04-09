// struct Demo <const U: usize> {
//   a: U,
//   b: u8,
// }

// fn demo<const I: usize>() {}

type TupleOf<T, N extends number, R extends unknown[] = []> =
  R['length'] extends N ? R : TupleOf<T, N, [...R, T]>;

export type Demo<U extends number> = { a: U, b: TupleOf<number, U> };

type A = Demo<1>;
type B = Demo<4>;

export type Demo2<U = string> = { a: U };

type AA = Demo2;
type BB = Demo2<number>;
