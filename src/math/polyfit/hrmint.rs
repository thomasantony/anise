/* hrmint.f -- translated by f2c (version 19980913).
   You must link the resulting object file with the libraries:
        -lf2c -lm   (in that order)
*/

/* $Procedure HRMINT ( Hermite polynomial interpolation  ) */
/* Subroutine */
pub fn hrmint_(xvals: &[f64], yvals: &[f64], x: f64) -> (f64, f64) {
    let work: &mut [f64] = &mut [0.0; 256];
    let n: usize = xvals.len();

    /* System generated locals */
    let work_dim1: usize;
    let work_offset: usize;

    /* Local variables */
    let mut temp: f64;
    let mut this__: usize;
    let mut prev: usize;
    let mut next: usize;

    /* Parameter adjustments */
    work_dim1 = n * 2;
    work_offset = work_dim1 + 1;

    assert!(n > 1);

    /*     Copy the input array into WORK.  After this, the first column */
    /*     of WORK represents the first column of our triangular */
    /*     interpolation table. */

    for i__ in 1..=n * 2 {
        work[i__ + work_dim1 - work_offset] = yvals[i__ - 1];
    }

    /*     Compute the second column of the interpolation table: this */
    /*     consists of the N-1 values obtained by evaluating the */
    /*     first-degree interpolants at X. We'll also evaluate the */
    /*     derivatives of these interpolants at X and save the results in */
    /*     the second column of WORK. Because the derivative computations */
    /*     depend on the function computations from the previous column in */
    /*     the interpolation table, and because the function interpolation */
    /*     overwrites the previous column of interpolated function values, */
    /*     we must evaluate the derivatives first. */

    for i__ in 1..=n - 1 {
        let c1 = xvals[i__] - x;
        let c2 = x - xvals[i__ - 1];
        let denom = xvals[i__] - xvals[i__ - 1];

        /*        The second column of WORK contains interpolated derivative */
        /*        values. */

        /*        The odd-indexed interpolated derivatives are simply the input */
        /*        derivatives. */

        prev = (i__ * 2) - 1;
        this__ = prev + 1;
        next = this__ + 1;
        work[prev + (work_dim1 * 2) - work_offset] = work[this__ + work_dim1 - work_offset];

        /*        The even-indexed interpolated derivatives are the slopes of */
        /*        the linear interpolating polynomials for adjacent input */
        /*        abscissa/ordinate pairs. */

        work[this__ + (work_dim1 * 2) - work_offset] =
            (work[next + work_dim1 - work_offset] - work[prev + work_dim1 - work_offset]) / denom;

        /*        The first column of WORK contains interpolated function values. */
        /*        The odd-indexed entries are the linear Taylor polynomials, */
        /*        for each input abscissa value, evaluated at X. */

        temp = work[this__ + work_dim1 - work_offset] * (x - xvals[i__ - 1])
            + work[prev + work_dim1 - work_offset];
        work[this__ + work_dim1 - work_offset] = (c1 * work[prev + work_dim1 - work_offset]
            + c2 * work[next + work_dim1 - work_offset])
            / denom;
        work[prev + work_dim1 - work_offset] = temp;
    }

    /*     The last column entries were not computed by the preceding loop; */
    /*     compute them now. */

    work[(n * 2) - 1 + (work_dim1 * 2) - work_offset] = work[(n * 2) + work_dim1 - work_offset];
    work[(n * 2) - 1 + work_dim1 - work_offset] = work[(n * 2) + work_dim1 - work_offset]
        * (x - xvals[n - 1])
        + work[(n * 2) - 1 + work_dim1 - work_offset];

    /*     Compute columns 3 through 2*N of the table. */

    for j in 2..=(n * 2) - 1 {
        for i in 1..=(n * 2) - j {
            /*           In the theoretical construction of the interpolation table,
             */
            /*           there are 2*N abscissa values, since each input abcissa */
            /*           value occurs with multiplicity two. In this theoretical */
            /*           construction, the Jth column of the interpolation table */
            /*           contains results of evaluating interpolants that span J+1 */
            /*           consecutive abscissa values.  The indices XI and XIJ below */
            /*           are used to pick the correct abscissa values out of the */
            /*           physical XVALS array, in which the abscissa values are not */
            /*           repeated. */

            let xi = (i + 1) / 2;
            let xij = (i + j + 1) / 2;
            let c1 = xvals[xij - 1] - x;
            let c2 = x - xvals[xi - 1];
            let denom = xvals[xij - 1] - xvals[xi - 1];

            /*           Compute the interpolated derivative at X for the Ith */
            /*           interpolant. This is the derivative with respect to X of */
            /*           the expression for the interpolated function value, which */
            /*           is the second expression below. This derivative computation
             */
            /*           is done first because it relies on the interpolated */
            /*           function values from the previous column of the */
            /*           interpolation table. */

            /*           The derivative expression here corresponds to equation */
            /*           2.35 on page 64 in reference [2]. */

            work[i + (work_dim1 * 2) - work_offset] = (c1
                * work[i + (work_dim1 * 2) - work_offset]
                + c2 * work[i + 1 + (work_dim1 * 2) - work_offset]
                + (work[i + 1 + work_dim1 - work_offset] - work[i + work_dim1 - work_offset]))
                / denom;

            /*           Compute the interpolated function value at X for the Ith */
            /*           interpolant. */

            work[i + work_dim1 - work_offset] = (c1 * work[i + work_dim1 - work_offset]
                + c2 * work[i + 1 + work_dim1 - work_offset])
                / denom;
        }
    }

    /*     Our interpolated function value is sitting in WORK(1,1) at this */
    /*     point.  The interpolated derivative is located in WORK(1,2). */

    let f = work[work_dim1 + 1 - work_offset];
    let df = work[(work_dim1 * 2) + 1 - work_offset];
    (f, df)
} /* hrmint_ */
