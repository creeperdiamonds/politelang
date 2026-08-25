//! Vectors and matrices.
//!
//! A vector is a list of numbers, and a matrix is a list of those lists. There is no new kind of
//! thing to learn — the rows really are rows, and you can look at one the way you look at any list.

use crate::numbers;
use crate::{Answer, Value};

fn numbers_of(items: &[Value]) -> Vec<f64> {
    items.iter().map(|v| v.as_decimal()).collect()
}

fn same_length(a: &[Value], b: &[Value]) -> Answer<()> {
    if a.len() != b.len() {
        return Err(format!(
            "one list holds {} and the other holds {}, so they cannot be paired up",
            a.len(),
            b.len()
        ));
    }
    Ok(())
}

/// Adding two lists item by item, keeping every number exact where it can be.
pub fn pairwise_sum(a: &[Value], b: &[Value]) -> Answer<Vec<Value>> {
    same_length(a, b)?;
    Ok(a.iter()
        .zip(b.iter())
        .map(|(x, y)| numbers::add(x, y))
        .collect())
}

pub fn pairwise_product(a: &[Value], b: &[Value]) -> Answer<Vec<Value>> {
    same_length(a, b)?;
    Ok(a.iter()
        .zip(b.iter())
        .map(|(x, y)| numbers::mul(x, y))
        .collect())
}

/// Every pair multiplied and the lot added together.
pub fn dot_product(a: &[Value], b: &[Value]) -> Answer<Value> {
    same_length(a, b)?;
    let mut total = Value::Whole(0);
    for (x, y) in a.iter().zip(b.iter()) {
        total = numbers::add(&total, &numbers::mul(x, y));
    }
    Ok(total)
}

/// The vector at right angles to both, which only exists in three dimensions.
pub fn cross_product(a: &[Value], b: &[Value]) -> Answer<Vec<Value>> {
    if a.len() != 3 || b.len() != 3 {
        return Err(
            "a cross product only means something for two lists of three, because it is the way \
             out of a plane"
                .to_string(),
        );
    }
    let m = |i: usize, j: usize| numbers::mul(&a[i], &b[j]);
    Ok(vec![
        numbers::sub(&m(1, 2), &m(2, 1)),
        numbers::sub(&m(2, 0), &m(0, 2)),
        numbers::sub(&m(0, 1), &m(1, 0)),
    ])
}

/// How long a vector is.
pub fn magnitude(a: &[Value]) -> Value {
    let total: f64 = numbers_of(a).iter().map(|n| n * n).sum();
    Value::Decimal(total.sqrt())
}

pub fn scaled_by(a: &[Value], factor: &Value) -> Vec<Value> {
    a.iter().map(|v| numbers::mul(v, factor)).collect()
}

// ---------------------------------------------------------------------------
// Matrices
// ---------------------------------------------------------------------------

/// Read a list of lists as rows of numbers, checking that it really is rectangular.
fn rows_of(m: &[Value]) -> Answer<Vec<Vec<f64>>> {
    if m.is_empty() {
        return Err("a matrix with no rows has nothing to work with".to_string());
    }
    let mut out: Vec<Vec<f64>> = Vec::with_capacity(m.len());
    let mut width: Option<usize> = None;
    for row in m {
        let items = match row {
            Value::List(items) => items.borrow().clone(),
            other => {
                return Err(format!(
                    "a matrix is a list of rows, and one of these is {}",
                    other.kind_name()
                ))
            }
        };
        match width {
            None => width = Some(items.len()),
            Some(w) if w != items.len() => {
                return Err(format!(
                    "one row holds {w} and another holds {}, so this is not a rectangle",
                    items.len()
                ))
            }
            _ => {}
        }
        out.push(numbers_of(&items));
    }
    Ok(out)
}

fn as_matrix(rows: Vec<Vec<f64>>) -> Vec<Value> {
    rows.into_iter()
        .map(|row| Value::list(row.into_iter().map(Value::Decimal).collect()))
        .collect()
}

pub fn matrix_product(a: &[Value], b: &[Value]) -> Answer<Vec<Value>> {
    let left = rows_of(a)?;
    let right = rows_of(b)?;
    let inner = left[0].len();
    if inner != right.len() {
        return Err(format!(
            "the first has {inner} across and the second has {} down, and those have to match",
            right.len()
        ));
    }
    let across = right[0].len();
    let mut out = vec![vec![0.0; across]; left.len()];
    for (i, row) in left.iter().enumerate() {
        for j in 0..across {
            let mut total = 0.0;
            for (k, value) in row.iter().enumerate() {
                total += value * right[k][j];
            }
            out[i][j] = total;
        }
    }
    Ok(as_matrix(out))
}

/// Turning a matrix on its side: rows become columns.
pub fn transpose(m: &[Value]) -> Answer<Vec<Value>> {
    let rows = rows_of(m)?;
    let across = rows[0].len();
    let mut out = vec![vec![0.0; rows.len()]; across];
    for (i, row) in rows.iter().enumerate() {
        for (j, value) in row.iter().enumerate() {
            out[j][i] = *value;
        }
    }
    Ok(as_matrix(out))
}

fn square(rows: &[Vec<f64>]) -> Answer<usize> {
    if rows.len() != rows[0].len() {
        return Err(format!(
            "this is {} down and {} across, and only a square one will do",
            rows.len(),
            rows[0].len()
        ));
    }
    Ok(rows.len())
}

/// How much a matrix stretches space. Zero means it flattens it, and cannot be undone.
pub fn determinant(m: &[Value]) -> Answer<Value> {
    let mut rows = rows_of(m)?;
    let n = square(&rows)?;

    // Gaussian elimination, keeping track of the sign as rows are swapped.
    let mut sign = 1.0;
    let mut total = 1.0;
    for i in 0..n {
        let mut pivot = i;
        for r in i..n {
            if rows[r][i].abs() > rows[pivot][i].abs() {
                pivot = r;
            }
        }
        if rows[pivot][i].abs() < 1e-12 {
            return Ok(Value::Decimal(0.0));
        }
        if pivot != i {
            rows.swap(pivot, i);
            sign = -sign;
        }
        total *= rows[i][i];
        for r in (i + 1)..n {
            let factor = rows[r][i] / rows[i][i];
            for c in i..n {
                rows[r][c] -= factor * rows[i][c];
            }
        }
    }
    Ok(Value::Decimal(sign * total))
}

/// The matrix that undoes this one.
pub fn matrix_inverse(m: &[Value]) -> Answer<Vec<Value>> {
    let rows = rows_of(m)?;
    let n = square(&rows)?;

    // Set the matrix beside an identity one, and work both down to the other.
    let mut work: Vec<Vec<f64>> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let mut wide = row.clone();
            for j in 0..n {
                wide.push(if i == j { 1.0 } else { 0.0 });
            }
            wide
        })
        .collect();

    for i in 0..n {
        let mut pivot = i;
        for r in i..n {
            if work[r][i].abs() > work[pivot][i].abs() {
                pivot = r;
            }
        }
        if work[pivot][i].abs() < 1e-12 {
            return Err(
                "this matrix flattens space, so there is no way of undoing it".to_string()
            );
        }
        work.swap(pivot, i);
        let divisor = work[i][i];
        for c in 0..(2 * n) {
            work[i][c] /= divisor;
        }
        for r in 0..n {
            if r == i {
                continue;
            }
            let factor = work[r][i];
            if factor == 0.0 {
                continue;
            }
            for c in 0..(2 * n) {
                work[r][c] -= factor * work[i][c];
            }
        }
    }

    Ok(as_matrix(
        work.into_iter().map(|row| row[n..].to_vec()).collect(),
    ))
}

/// The matrix that changes nothing.
pub fn identity_matrix(size: i64) -> Answer<Vec<Value>> {
    if size < 1 {
        return Err("a matrix needs at least one row".to_string());
    }
    if size > 1024 {
        return Err(format!("{size} rows is more than I should be asked to hold"));
    }
    let n = size as usize;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let row: Vec<Value> = (0..n)
            .map(|j| if i == j { Value::Whole(1) } else { Value::Whole(0) })
            .collect();
        out.push(Value::list(row));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(values: &[i64]) -> Vec<Value> {
        values.iter().map(|n| Value::Whole(*n)).collect()
    }

    fn matrix(rows: &[&[i64]]) -> Vec<Value> {
        rows.iter().map(|r| Value::list(v(r))).collect()
    }

    fn numbers_in(row: &Value) -> Vec<f64> {
        match row {
            Value::List(items) => items.borrow().iter().map(|x| x.as_decimal()).collect(),
            _ => Vec::new(),
        }
    }

    #[test]
    fn vectors_add_and_multiply_item_by_item() {
        let sum = pairwise_sum(&v(&[1, 2, 3]), &v(&[10, 20, 30])).unwrap();
        assert_eq!(sum.iter().map(|x| x.as_whole()).collect::<Vec<_>>(), vec![11, 22, 33]);
        let product = pairwise_product(&v(&[1, 2, 3]), &v(&[10, 20, 30])).unwrap();
        assert_eq!(product.iter().map(|x| x.as_whole()).collect::<Vec<_>>(), vec![10, 40, 90]);
        assert!(pairwise_sum(&v(&[1, 2]), &v(&[1])).is_err());
    }

    #[test]
    fn a_dot_product_stays_exact_for_whole_numbers() {
        let d = dot_product(&v(&[1, 2, 3]), &v(&[4, 5, 6])).unwrap();
        assert_eq!(d.as_whole(), 32);
        assert!(matches!(d, Value::Whole(_)), "should not become a decimal");
    }

    #[test]
    fn a_cross_product_points_out_of_the_plane() {
        let c = cross_product(&v(&[1, 0, 0]), &v(&[0, 1, 0])).unwrap();
        assert_eq!(c.iter().map(|x| x.as_whole()).collect::<Vec<_>>(), vec![0, 0, 1]);
        assert!(cross_product(&v(&[1, 2]), &v(&[3, 4])).is_err());
    }

    #[test]
    fn a_three_four_five_triangle_has_a_length_of_five() {
        assert_eq!(magnitude(&v(&[3, 4])).as_decimal(), 5.0);
        let scaled = scaled_by(&v(&[1, 2, 3]), &Value::Whole(3));
        assert_eq!(scaled.iter().map(|x| x.as_whole()).collect::<Vec<_>>(), vec![3, 6, 9]);
    }

    #[test]
    fn matrices_multiply_the_way_they_should() {
        let a = matrix(&[&[1, 2], &[3, 4]]);
        let b = matrix(&[&[5, 6], &[7, 8]]);
        let p = matrix_product(&a, &b).unwrap();
        assert_eq!(numbers_in(&p[0]), vec![19.0, 22.0]);
        assert_eq!(numbers_in(&p[1]), vec![43.0, 50.0]);
        // Shapes that do not meet are refused rather than guessed at.
        assert!(matrix_product(&a, &matrix(&[&[1, 2, 3]])).is_err());
    }

    #[test]
    fn a_matrix_turns_on_its_side() {
        let t = transpose(&matrix(&[&[1, 2, 3], &[4, 5, 6]])).unwrap();
        assert_eq!(t.len(), 3);
        assert_eq!(numbers_in(&t[0]), vec![1.0, 4.0]);
        assert_eq!(numbers_in(&t[2]), vec![3.0, 6.0]);
        // A ragged one is not a matrix at all.
        let ragged = vec![Value::list(v(&[1, 2])), Value::list(v(&[3]))];
        assert!(transpose(&ragged).is_err());
    }

    #[test]
    fn determinants_agree_with_the_textbook() {
        let d = determinant(&matrix(&[&[1, 2], &[3, 4]])).unwrap();
        assert!((d.as_decimal() + 2.0).abs() < 1e-12);
        let d = determinant(&matrix(&[&[2, 0], &[0, 3]])).unwrap();
        assert!((d.as_decimal() - 6.0).abs() < 1e-12);
        // A flattening one comes out at nothing.
        let d = determinant(&matrix(&[&[1, 2], &[2, 4]])).unwrap();
        assert!(d.as_decimal().abs() < 1e-12);
        assert!(determinant(&matrix(&[&[1, 2, 3], &[4, 5, 6]])).is_err());
    }

    #[test]
    fn an_inverse_undoes_its_matrix() {
        let a = matrix(&[&[4, 7], &[2, 6]]);
        let inverse = matrix_inverse(&a).unwrap();
        let back = matrix_product(&a, &inverse).unwrap();
        assert!((numbers_in(&back[0])[0] - 1.0).abs() < 1e-9);
        assert!(numbers_in(&back[0])[1].abs() < 1e-9);
        assert!((numbers_in(&back[1])[1] - 1.0).abs() < 1e-9);
        // One that flattens space cannot be undone, and says so.
        assert!(matrix_inverse(&matrix(&[&[1, 2], &[2, 4]])).is_err());
    }

    #[test]
    fn the_identity_matrix_changes_nothing() {
        let i = identity_matrix(3).unwrap();
        assert_eq!(numbers_in(&i[0]), vec![1.0, 0.0, 0.0]);
        assert_eq!(numbers_in(&i[2]), vec![0.0, 0.0, 1.0]);
        let a = matrix(&[&[1, 2, 3], &[4, 5, 6], &[7, 8, 9]]);
        let same = matrix_product(&a, &i).unwrap();
        assert_eq!(numbers_in(&same[1]), vec![4.0, 5.0, 6.0]);
        assert!(identity_matrix(0).is_err());
    }
}
