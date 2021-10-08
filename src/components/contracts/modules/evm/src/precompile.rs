use crate::runtime::stack::FindoraStackState;
use ethereum_types::H160;
use evm::{executor::PrecompileOutput, Context, ExitError, ExitSucceed};
use impl_trait_for_tuples::impl_for_tuples;

pub use fp_core::context::Context as FinState;

/// Custom precompiles to be used by EVM engine.
pub trait PrecompileSet {
    /// Try to execute the code address as precompile. If the code address is not
    /// a precompile or the precompile is not yet available, return `None`.
    /// Otherwise, calculate the amount of gas needed with given `input` and
    /// `target_gas`. Return `Some(Ok(status, output, gas_used))` if the execution
    /// is successful. Otherwise return `Some(Err(_))`.
    fn execute<'context, 'vicinity, 'config, T>(
        address: H160,
        input: &[u8],
        target_gas: Option<u64>,
        context: &Context,
        state: &mut FindoraStackState<'context, 'vicinity, 'config, T>,
        is_static: bool,
    ) -> Option<core::result::Result<PrecompileOutput, ExitError>>;
}

/// One single precompile used by EVM engine.
pub trait Precompile {
    /// Try to execute the precompile. Calculate the amount of gas needed with given `input` and
    /// `target_gas`. Return `Ok(status, output, gas_used)` if the execution is
    /// successful. Otherwise return `Err(_)`.
    fn execute(
        input: &[u8],
        target_gas: Option<u64>,
        context: &Context,
        state: &FinState,
    ) -> core::result::Result<PrecompileOutput, ExitError>;
}

#[impl_for_tuples(16)]
#[tuple_types_no_default_trait_bound]
impl PrecompileSet for Tuple {
    for_tuples!( where #( Tuple: Precompile )* );

    fn execute<'context, 'vicinity, 'config, T>(
        address: H160,
        input: &[u8],
        target_gas: Option<u64>,
        context: &Context,
        state: &mut FindoraStackState<'context, 'vicinity, 'config, T>,
        _is_static: bool,
    ) -> Option<core::result::Result<PrecompileOutput, ExitError>> {
        let mut index = 0;

        for_tuples!( #(
			index += 1;
			if address == H160::from_low_u64_be(index) {
				return Some(Tuple::execute(input, target_gas, context, state.ctx))
			}
		)* );

        None
    }
}

pub trait LinearCostPrecompile {
    const BASE: u64;
    const WORD: u64;

    fn execute(
        input: &[u8],
        cost: u64,
    ) -> core::result::Result<(ExitSucceed, Vec<u8>), ExitError>;
}

impl<T: LinearCostPrecompile> Precompile for T {
    fn execute(
        input: &[u8],
        target_gas: Option<u64>,
        _: &Context,
        _: &FinState,
    ) -> core::result::Result<PrecompileOutput, ExitError> {
        let cost = ensure_linear_cost(target_gas, input.len() as u64, T::BASE, T::WORD)?;

        let (exit_status, output) = T::execute(input, cost)?;
        Ok(PrecompileOutput {
            exit_status,
            cost,
            output,
            logs: Default::default(),
        })
    }
}

/// Linear gas cost
fn ensure_linear_cost(
    target_gas: Option<u64>,
    len: u64,
    base: u64,
    word: u64,
) -> Result<u64, ExitError> {
    let cost = base
        .checked_add(
            word.checked_mul(len.saturating_add(31) / 32)
                .ok_or(ExitError::OutOfGas)?,
        )
        .ok_or(ExitError::OutOfGas)?;

    if let Some(target_gas) = target_gas {
        if cost > target_gas {
            return Err(ExitError::OutOfGas);
        }
    }

    Ok(cost)
}