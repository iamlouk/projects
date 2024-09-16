#![allow(non_snake_case)]

use std::cell::RefCell;
use std::ops::{Add, AddAssign, Mul, Sub};
use std::rc::Rc;

pub trait Scalar:
    Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> + AddAssign + Copy + From<u8>
{
}

pub trait Mat<T: Scalar>: Clone {
    fn rows(&self) -> usize;
    fn cols(&self) -> usize;
    fn get(&self, i: usize, j: usize) -> T;

    fn smul(&self, rhs: T) -> MatrixMulScalar<T, Self>;
    fn mmul<R: Mat<T>>(&self, rhs: &R) -> MatrixMul<T, Self, R>;
    fn madd<R: Mat<T>>(&self, rhs: &R) -> MatrixAdd<T, Self, R>;
}

#[derive(Clone, Debug)]
pub struct Matrix<T: Scalar> {
    data: Rc<RefCell<Vec<T>>>,
    mrows: usize,
    mcols: usize,
}

impl<T: Scalar, Rhs: Mat<T>> Add<Rhs> for Matrix<T> {
    type Output = MatrixAdd<T, Self, Rhs>;

    fn add(self, rhs: Rhs) -> Self::Output {
        Self::Output {
            lhs: self,
            rhs,
            _pd: std::marker::PhantomData,
        }
    }
}

impl<T: Scalar, Rhs: Mat<T>> Mul<Rhs> for Matrix<T> {
    type Output = MatrixMul<T, Self, Rhs>;

    fn mul(self, rhs: Rhs) -> Self::Output {
        Self::Output {
            lhs: self,
            rhs,
            _pd: std::marker::PhantomData,
        }
    }
}

impl<T: Scalar> Matrix<T> {
    pub fn zero(n: usize, m: usize) -> Self {
        Matrix {
            data: Rc::new(RefCell::new(vec![T::from(0); n * m])),
            mrows: n,
            mcols: m,
        }
    }

    pub fn from<M: Mat<T>>(mat: &M) -> Self {
        let rows = mat.rows();
        let cols = mat.cols();
        let mut val = Self::zero(rows, cols);
        for i in 0..rows {
            for j in 0..cols {
                val.set(i, j, mat.get(i, j));
            }
        }
        val
    }

    pub fn set(&mut self, i: usize, j: usize, x: T) {
        assert!(i < self.rows() && j < self.cols());
        self.data.borrow_mut()[i * self.cols() + j] = x;
    }
}

impl<T: Scalar> Mat<T> for Matrix<T> {
    fn rows(&self) -> usize {
        self.mrows
    }
    fn cols(&self) -> usize {
        self.mcols
    }
    fn get(&self, i: usize, j: usize) -> T {
        assert!(i < self.rows() && j < self.cols());
        self.data.borrow()[i * self.mcols + j]
    }

    fn smul(&self, rhs: T) -> MatrixMulScalar<T, Self> {
        MatrixMulScalar {
            lhs: self.clone(),
            rhs,
        }
    }
    fn mmul<R: Mat<T>>(&self, rhs: &R) -> MatrixMul<T, Self, R> {
        assert!(self.cols() == rhs.rows());
        MatrixMul {
            lhs: self.clone(),
            rhs: rhs.clone(),
            _pd: std::marker::PhantomData,
        }
    }
    fn madd<RHS: Mat<T>>(&self, rhs: &RHS) -> MatrixAdd<T, Self, RHS> {
        assert!(self.rows() == rhs.rows() && self.cols() == rhs.cols());
        MatrixAdd {
            lhs: self.clone(),
            rhs: rhs.clone(),
            _pd: std::marker::PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct MatrixAdd<T: Scalar, L: Mat<T>, R: Mat<T>> {
    lhs: L,
    rhs: R,
    _pd: std::marker::PhantomData<T>,
}

impl<T: Scalar, Rhs: Mat<T>, MyLhs: Mat<T>, MyRhs: Mat<T>> Add<Rhs> for MatrixAdd<T, MyLhs, MyRhs> {
    type Output = MatrixAdd<T, Self, Rhs>;

    fn add(self, rhs: Rhs) -> Self::Output {
        Self::Output {
            lhs: self,
            rhs,
            _pd: std::marker::PhantomData,
        }
    }
}

impl<T: Scalar, Rhs: Mat<T>, MyLhs: Mat<T>, MyRhs: Mat<T>> Mul<Rhs> for MatrixAdd<T, MyLhs, MyRhs> {
    type Output = MatrixMul<T, Self, Rhs>;

    fn mul(self, rhs: Rhs) -> Self::Output {
        Self::Output {
            lhs: self,
            rhs,
            _pd: std::marker::PhantomData,
        }
    }
}

impl<T: Scalar, L: Mat<T>, R: Mat<T>> Mat<T> for MatrixAdd<T, L, R> {
    fn rows(&self) -> usize {
        self.lhs.rows()
    }
    fn cols(&self) -> usize {
        self.lhs.cols()
    }
    fn get(&self, i: usize, j: usize) -> T {
        self.lhs.get(i, j) + self.rhs.get(i, j)
    }

    fn smul(&self, rhs: T) -> MatrixMulScalar<T, Self> {
        MatrixMulScalar {
            lhs: self.clone(),
            rhs,
        }
    }
    fn mmul<RHS: Mat<T>>(&self, rhs: &RHS) -> MatrixMul<T, Self, RHS> {
        assert!(self.cols() == rhs.rows());
        MatrixMul {
            lhs: self.clone(),
            rhs: rhs.clone(),
            _pd: std::marker::PhantomData,
        }
    }
    fn madd<RHS: Mat<T>>(&self, rhs: &RHS) -> MatrixAdd<T, Self, RHS> {
        assert!(self.rows() == rhs.rows() && self.cols() == rhs.cols());
        MatrixAdd {
            lhs: self.clone(),
            rhs: rhs.clone(),
            _pd: std::marker::PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct MatrixMulScalar<T: Scalar, L: Mat<T>> {
    lhs: L,
    rhs: T,
}

impl<T: Scalar, L: Mat<T>> Mat<T> for MatrixMulScalar<T, L> {
    fn rows(&self) -> usize {
        self.lhs.rows()
    }
    fn cols(&self) -> usize {
        self.lhs.cols()
    }
    fn get(&self, i: usize, j: usize) -> T {
        self.lhs.get(i, j) * self.rhs
    }

    fn smul(&self, rhs: T) -> MatrixMulScalar<T, Self> {
        MatrixMulScalar {
            lhs: self.clone(),
            rhs,
        }
    }
    fn mmul<R: Mat<T>>(&self, rhs: &R) -> MatrixMul<T, Self, R> {
        assert!(self.cols() == rhs.rows());
        MatrixMul {
            lhs: self.clone(),
            rhs: rhs.clone(),
            _pd: std::marker::PhantomData,
        }
    }
    fn madd<RHS: Mat<T>>(&self, rhs: &RHS) -> MatrixAdd<T, Self, RHS> {
        assert!(self.rows() == rhs.rows() && self.cols() == rhs.cols());
        MatrixAdd {
            lhs: self.clone(),
            rhs: rhs.clone(),
            _pd: std::marker::PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct MatrixMul<T: Scalar, L: Mat<T>, R: Mat<T>> {
    lhs: L,
    rhs: R,
    _pd: std::marker::PhantomData<T>,
}

impl<T: Scalar, Rhs: Mat<T>, MyLhs: Mat<T>, MyRhs: Mat<T>> Add<Rhs> for MatrixMul<T, MyLhs, MyRhs> {
    type Output = MatrixAdd<T, Self, Rhs>;

    fn add(self, rhs: Rhs) -> Self::Output {
        Self::Output {
            lhs: self,
            rhs,
            _pd: std::marker::PhantomData,
        }
    }
}

impl<T: Scalar, Rhs: Mat<T>, MyLhs: Mat<T>, MyRhs: Mat<T>> Mul<Rhs> for MatrixMul<T, MyLhs, MyRhs> {
    type Output = MatrixMul<T, Self, Rhs>;

    fn mul(self, rhs: Rhs) -> Self::Output {
        Self::Output {
            lhs: self,
            rhs,
            _pd: std::marker::PhantomData,
        }
    }
}

impl<T: Scalar, L: Mat<T>, R: Mat<T>> Mat<T> for MatrixMul<T, L, R> {
    fn rows(&self) -> usize {
        self.lhs.rows()
    }
    fn cols(&self) -> usize {
        self.rhs.cols()
    }
    fn get(&self, i: usize, j: usize) -> T {
        assert!(self.lhs.cols() == self.rhs.rows());
        let mut x: T = T::from(0);
        for k in 0..self.lhs.cols() {
            x += self.lhs.get(i, k) * self.rhs.get(k, j);
        }
        x
    }

    fn smul(&self, rhs: T) -> MatrixMulScalar<T, Self> {
        MatrixMulScalar {
            lhs: self.clone(),
            rhs,
        }
    }
    fn mmul<RHS: Mat<T>>(&self, rhs: &RHS) -> MatrixMul<T, Self, RHS> {
        assert!(self.cols() == rhs.rows());
        MatrixMul {
            lhs: self.clone(),
            rhs: rhs.clone(),
            _pd: std::marker::PhantomData,
        }
    }
    fn madd<RHS: Mat<T>>(&self, rhs: &RHS) -> MatrixAdd<T, Self, RHS> {
        assert!(self.rows() == rhs.rows() && self.cols() == rhs.cols());
        MatrixAdd {
            lhs: self.clone(),
            rhs: rhs.clone(),
            _pd: std::marker::PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_ops() {
        impl Scalar for f32 {}

        let A = Matrix::<f32>::zero(10, 15);
        let B = Matrix::<f32>::zero(15, 5);

        let C = (A.clone() + A) * B;

        assert!(C.rows() == 10 && C.cols() == 5);

        /*
        for i in 0..10 {
            for j in 0..15 {
                A.set(i, j, (i + j) as f32);
            }
        }

        for i in 0..15 {
            for j in 0..5 {
                B.set(i, j, (i * j) as f32);
            }
        }

        let C = Matrix::from(&A.madd(&A).smul(2.0f32).mmul(&B));

        assert!(C.rows() == 10 && C.cols() == 5);
        std::eprintln!("C: {:?}", C)
        */
    }
}
