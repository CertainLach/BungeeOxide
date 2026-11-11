use std::cell::Cell;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TooDeep;

thread_local!(static FLOOR: Cell<usize> = const { Cell::new(0) });

psm::psm_stack_information! {
	yes {
		#[inline(always)]
		fn sp() -> usize {
			psm::stack_pointer() as usize
		}
	}
	no {
		#[inline(always)]
		fn sp() -> usize {
			let probe = 0u8;
			std::hint::black_box(&probe) as *const u8 as usize
		}
	}
}

struct Restore(usize);
impl Drop for Restore {
	fn drop(&mut self) {
		FLOOR.set(self.0);
	}
}

pub fn budget<R>(bytes: usize, f: impl FnOnce() -> R) -> R {
	let prev = FLOOR.get();
	let _restore = Restore(prev);
	FLOOR.set(prev.max(sp().saturating_sub(bytes)));
	f()
}

#[inline(always)]
pub fn check() -> Result<(), TooDeep> {
	if sp() < FLOOR.get() {
		Err(TooDeep)
	} else {
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn dive(n: usize) -> Result<usize, TooDeep> {
		check()?;
		let pad = std::hint::black_box([0u8; 256]);
		if n == 0 {
			return Ok(pad[0] as usize);
		}
		Ok(dive(n - 1)? + 1)
	}

	#[test]
	fn unbounded_without_budget() {
		assert_eq!(dive(100), Ok(100));
	}

	#[test]
	fn budget_stops_deep_recursion() {
		assert_eq!(budget(64 << 10, || dive(1 << 20)), Err(TooDeep));
		assert_eq!(budget(64 << 10, || dive(10)), Ok(10));
		assert_eq!(dive(100), Ok(100));
	}

	#[test]
	fn nested_budget_only_tightens() {
		budget(16 << 10, || {
			assert_eq!(budget(1 << 30, || dive(1 << 20)), Err(TooDeep));
		});
	}
}
