#![doc(hidden)]

//! Minimal `SQLx` API surface used by `SQLx` lint UI tests and resolution checks.
//!
//! The fixture provides stable stand-ins for query, row, and safety APIs so
//! tests can exercise semantic lint behavior without a database or runtime.
//! Each item intentionally mirrors a small public `SQLx` contract while keeping
//! behavior deterministic, dependency-light, and suitable for compile-fail UI
//! fixtures that focus on one lint diagnostic at a time.

/// Assert that a dynamically constructed SQL string has been audited for injection.
///
/// The wrapper preserves the inner expression type while providing the exact
/// constructor shape that the `SQLx` safety lint resolves during analysis.
#[derive(Clone, Copy, Debug)]
pub struct AssertSqlSafe<T>(pub T);

/// A database row that supports typed value access in the fixture API.
///
/// Implementations may use any storage because these methods exist to provide
/// stable semantic targets for private `SQLx` lints and their UI examples.
pub trait Row {
    /// Decode a typed value, panicking when the column or requested type is invalid.
    /// The default fixture implementation returns a deterministic value for every
    /// index so UI tests can focus on lint resolution rather than database behavior.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sqlx::{Row, TestRow};
    ///
    /// let value: u32 = TestRow.get(0);
    /// assert_eq!(value, 0);
    /// ```
    fn get<T: Default, I>(&self, _index: I) -> T {
        T::default()
    }

    /// Decode without compatibility checking, panicking on failure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sqlx::{Row, TestRow};
    ///
    /// let value: u32 = TestRow.get_unchecked(0);
    /// assert_eq!(value, 0);
    /// ```
    fn get_unchecked<T: Default, I>(&self, _index: I) -> T {
        T::default()
    }

    /// Decode a value without panicking.
    ///
    /// # Errors
    ///
    /// Returns an error when the column or requested type is invalid.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sqlx::{Row, TestRow};
    ///
    /// let value = TestRow.try_get::<u32, _>(0);
    /// assert!(value.is_ok());
    /// ```
    fn try_get<T: Default, I>(&self, _index: I) -> Result<T, Error> {
        Ok(T::default())
    }

    /// Decode without compatibility checking or panicking.
    ///
    /// # Errors
    ///
    /// Returns an error when the column is invalid or decoding fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sqlx::{Row, TestRow};
    ///
    /// let value = TestRow.try_get_unchecked::<u32, _>(0);
    /// assert!(value.is_ok());
    /// ```
    fn try_get_unchecked<T: Default, I>(&self, _index: I) -> Result<T, Error> {
        Ok(T::default())
    }
}

/// A concrete row for UI examples.
#[derive(Clone, Copy, Debug)]
pub struct TestRow;

impl Row for TestRow {}

/// A pooled connection.
#[derive(Clone, Copy, Debug)]
pub struct PoolConnection;

impl PoolConnection {
    /// Permanently remove the connection from its pool.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sqlx::PoolConnection;
    ///
    /// let _connection = PoolConnection.leak();
    /// ```
    pub const fn leak(self) -> Connection {
        Connection
    }

    /// Detach the connection while allowing the pool to replace it.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sqlx::PoolConnection;
    ///
    /// let _connection = PoolConnection.detach();
    /// ```
    pub const fn detach(self) -> Connection {
        Connection
    }
}

/// A detached database connection.
#[derive(Clone, Copy, Debug)]
pub struct Connection;

/// A minimal decoding error.
#[derive(Clone, Copy, Debug)]
pub struct Error;

/// Minimal dynamic SQL builder.
#[derive(Clone, Copy, Debug, Default)]
pub struct QueryBuilder;

impl QueryBuilder {
    /// Append SQL.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sqlx::QueryBuilder;
    ///
    /// let mut builder = QueryBuilder;
    /// let _builder = builder.push("SELECT 1");
    /// ```
    pub fn push(&mut self, _sql: impl std::fmt::Display) -> &mut Self {
        self
    }

    /// Start a separated SQL list.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sqlx::QueryBuilder;
    ///
    /// let mut builder = QueryBuilder;
    /// let _values = builder.separated(", ");
    /// ```
    pub fn separated(&mut self, _separator: impl std::fmt::Display) -> Separated {
        Separated
    }

    /// Append a values list.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sqlx::QueryBuilder;
    ///
    /// let mut builder = QueryBuilder;
    /// let _builder = builder.push_values([1, 2, 3]);
    /// ```
    pub fn push_values<T>(&mut self, _values: impl IntoIterator<Item = T>) -> &mut Self {
        self
    }

    /// Append a tuple list.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sqlx::QueryBuilder;
    ///
    /// let mut builder = QueryBuilder;
    /// let _builder = builder.push_tuples([(1, 2), (3, 4)]);
    /// ```
    pub fn push_tuples<T>(&mut self, _values: impl IntoIterator<Item = T>) -> &mut Self {
        self
    }
}

/// Minimal separated SQL list.
#[derive(Clone, Copy, Debug, Default)]
pub struct Separated;

impl Separated {
    /// Append SQL without first adding a separator.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sqlx::Separated;
    ///
    /// let mut values = Separated;
    /// let _values = values.push_unseparated("NULL");
    /// ```
    pub fn push_unseparated(&mut self, _sql: impl std::fmt::Display) -> &mut Self {
        self
    }
}

/// Minimal pool options builder.
#[derive(Clone, Copy, Debug, Default)]
pub struct PoolOptions;

impl PoolOptions {
    /// Set the maximum pool size.
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sqlx::PoolOptions;
    ///
    /// let _options = PoolOptions.max_connections(5);
    /// ```
    pub const fn max_connections(self, _value: u32) -> Self {
        self
    }
}

/// Minimal prepared statement.
pub trait Statement {
    /// Index a column, panicking when it is absent.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sqlx::{Statement, TestStatement};
    ///
    /// TestStatement.column(0);
    /// ```
    fn column(&self, _index: usize) {}

    /// Index a column without panicking.
    ///
    /// # Errors
    ///
    /// Returns an error when the column is absent.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sqlx::{Statement, TestStatement};
    ///
    /// assert!(TestStatement.try_column(0).is_ok());
    /// ```
    fn try_column(&self, _index: usize) -> Result<(), Error> {
        Ok(())
    }
}

/// Concrete statement for fixtures.
#[derive(Clone, Copy, Debug)]
pub struct TestStatement;

impl Statement for TestStatement {}

/// Stand-in for `SQLx`'s checked query macro.
#[macro_export]
macro_rules! query {
    ($($tokens:tt)*) => {
        ()
    };
}

/// Stand-in for `SQLx`'s unchecked query macro.
#[macro_export]
macro_rules! query_unchecked {
    ($($tokens:tt)*) => {
        ()
    };
}

/// Stand-in for `SQLx`'s checked query-as macro.
#[macro_export]
macro_rules! query_as {
    ($($tokens:tt)*) => {
        ()
    };
}

/// Stand-in for `SQLx`'s unchecked query-as macro.
#[macro_export]
macro_rules! query_as_unchecked {
    ($($tokens:tt)*) => {
        ()
    };
}

/// Stand-in for `SQLx`'s checked query-file macro.
#[macro_export]
macro_rules! query_file {
    ($($tokens:tt)*) => {
        ()
    };
}

/// Stand-in for `SQLx`'s unchecked query-file macro.
#[macro_export]
macro_rules! query_file_unchecked {
    ($($tokens:tt)*) => {
        ()
    };
}

/// Stand-in for `SQLx`'s checked query-file-as macro.
#[macro_export]
macro_rules! query_file_as {
    ($($tokens:tt)*) => {
        ()
    };
}

/// Stand-in for `SQLx`'s unchecked query-file-as macro.
#[macro_export]
macro_rules! query_file_as_unchecked {
    ($($tokens:tt)*) => {
        ()
    };
}

/// Stand-in for `SQLx`'s checked scalar query macro.
#[macro_export]
macro_rules! query_scalar {
    ($($tokens:tt)*) => {
        ()
    };
}

/// Stand-in for `SQLx`'s unchecked scalar query macro.
#[macro_export]
macro_rules! query_scalar_unchecked {
    ($($tokens:tt)*) => {
        ()
    };
}

/// Stand-in for `SQLx`'s checked scalar query-file macro.
#[macro_export]
macro_rules! query_file_scalar {
    ($($tokens:tt)*) => {
        ()
    };
}

/// Stand-in for `SQLx`'s unchecked scalar query-file macro.
#[macro_export]
macro_rules! query_file_scalar_unchecked {
    ($($tokens:tt)*) => {
        ()
    };
}
