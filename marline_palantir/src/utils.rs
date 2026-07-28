use num::integer::gcd;

pub fn lcm_checked(a: u32, b: u32) -> Option<u32> {
    let gcd_val = gcd(a, b);
    (a / gcd_val).checked_mul(b)
}

pub fn lcm_vec(nums: &[u32]) -> Option<u32> {
    let mut res: u32 = 1;
    for &i in nums {
        res = lcm_checked(res, i)?;
    }
    Some(res)
}
