// compile-flags: --edition 2024

#![allow(dead_code, unused_variables)]

fn consume<T>(_value: T) {}

#[derive(Clone, Copy)]
struct Feature {
    is_enabled: bool,
}

struct SourceError;
struct WrappedError(SourceError);

struct Client;

impl Client {
    async fn complicated(&self) -> Result<Feature, SourceError> {
        Ok(Feature { is_enabled: true })
    }

    fn feature(&self) -> Feature {
        Feature { is_enabled: true }
    }

    fn is_enabled(&self) -> bool {
        true
    }

    fn wrapper(&self) -> NonBoolHelper {
        NonBoolHelper
    }
}

struct HelloWorld;

impl HelloWorld {
    fn is_something_enabled(&self) -> bool {
        true
    }

    fn is_ready(&self) -> bool {
        true
    }
}

struct Config {
    hello_world: HelloWorld,
    is_enabled: bool,
}

impl Config {
    fn owner(&self) -> &Config {
        self
    }
}

struct Account {
    profile: Config,
}

impl Account {
    fn refresh(&self) -> Result<Feature, SourceError> {
        Ok(Feature { is_enabled: true })
    }

    fn owner(&self) -> &Config {
        &self.profile
    }

    fn is_open(&self) -> bool {
        true
    }

    fn wrapper(&self) -> NonBoolHelper {
        NonBoolHelper
    }
}

struct NonBoolHelper;

impl NonBoolHelper {
    fn and(self, _other: Self) -> Self {
        self
    }
}

fn bool_helper<T>(_value: T) -> bool {
    true
}

macro_rules! complex_bool {
    ($account:expr, $config:expr) => {
        $account.owner().hello_world.is_something_enabled() && $config.hello_world.is_ready()
    };
}

async fn warn_fallible_async_chain(client: &Client, account: &Account) -> Result<(), WrappedError> {
    if client.complicated().await.map_err(WrappedError)?.is_enabled
        && account
            .refresh()
            .map(|feature| feature.is_enabled)
            .unwrap_or(false)
    {
        consume(());
    }

    Ok(())
}

fn warn_two_complex_terms(account: &Account) {
    if account.owner().owner().hello_world.is_something_enabled()
        && account
            .refresh()
            .map(|feature| feature.is_enabled)
            .unwrap_or(false)
    {
        consume(());
    }
}

fn warn_mixed_boolean_groups(account: &Account, config: &Config) {
    if account.owner().owner().hello_world.is_something_enabled()
        || (config.hello_world.is_ready() && account.refresh().is_ok())
    {
        consume(());
    }
}

fn warn_bool_block_tail(client: &Client, account: &Account) {
    if { client.feature().is_enabled && client.is_enabled() }
        && account.owner().hello_world.is_something_enabled()
    {
        consume(());
    }
}

fn keep_fluent_field_chains(account: &Account, config: &Config) {
    if account.owner().hello_world.is_something_enabled() && config.hello_world.is_ready() {
        consume(());
    }
}

fn keep_context_field_chains(account: &Account, config: &Config) {
    if account.profile.hello_world.is_something_enabled()
        && config.hello_world.is_ready()
        && account.profile.is_enabled
    {
        consume(());
    }
}

fn keep_trivial_closure(values: &[Option<u64>], account: &Account) {
    if account.is_open() && values.first().is_some_and(|value| value.is_some()) {
        consume(());
    }
}

fn keep_simple_predicates(client: &Client, account: &Account, count: usize) {
    if client.is_enabled() && account.is_open() && count > 0 {
        consume(());
    }
}

fn keep_named_predicates(is_enabled: bool, is_other_enabled: bool) {
    if is_enabled && is_other_enabled {
        consume(());
    }
}

fn keep_single_complex_term(client: &Client) {
    if client.feature().is_enabled {
        consume(());
    }
}

fn keep_non_bool_helper_arguments(client: &Client, account: &Account) {
    if bool_helper(client.wrapper().and(account.wrapper()))
        && bool_helper(account.wrapper().and(client.wrapper()))
    {
        consume(());
    }
}

fn keep_non_bool_return_expression(client: &Client, account: &Account) -> bool {
    if bool_helper({
        if client.is_enabled() {
            return account.is_open();
        }

        client.wrapper().and(account.wrapper())
    }) && bool_helper(account.wrapper().and(client.wrapper()))
    {
        return true;
    }

    false
}

fn keep_nested_simple_conditions(client: &Client, account: &Account) {
    if client.is_enabled() {
        if account.is_open() {
            consume(());
        }
    }
}

fn keep_macro_condition(client: &Client, account: &Account) {
    if cfg!(debug_assertions) && client.feature().is_enabled && account.profile.is_enabled {
        consume(());
    }
}

fn keep_macro_expanded_condition(account: &Account, config: &Config) {
    if complex_bool!(account, config) {
        consume(());
    }
}

#[cfg(any())]
fn keep_cfg_disabled_condition(account: &Account, config: &Config) {
    if account.owner().hello_world.is_something_enabled() && config.hello_world.is_ready() {
        consume(());
    }
}

fn warn_while_condition(account: &Account) {
    while account.owner().owner().hello_world.is_something_enabled()
        && account
            .refresh()
            .map(|feature| feature.is_enabled)
            .unwrap_or(false)
    {
        break;
    }
}

fn main() {}
