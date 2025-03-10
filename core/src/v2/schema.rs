//! Traits used for code and spec generation.

use super::models::{
    DataType, DataTypeFormat, DefaultOperationRaw, DefaultSchemaRaw, Either, Resolvable,
    SecurityScheme,
};

use std::collections::{BTreeMap, BTreeSet};

/// Interface for the [`Schema`](https://github.com/OAI/OpenAPI-Specification/blob/master/versions/2.0.md#schemaObject) object.
///
/// This is only used for resolving the definitions.
///
/// **NOTE:** Don't implement this by yourself! Please use the `#[api_v2_schema]`
/// proc macro attribute instead.
pub trait Schema: Sized {
    /// Description for this schema, if any (`description` field).
    fn description(&self) -> Option<&str>;

    /// Reference to some other schema, if any (`$ref` field).
    fn reference(&self) -> Option<&str>;

    /// Data type of this schema, if any (`type` field).
    fn data_type(&self) -> Option<DataType>;

    /// Data type format used by this schema, if any (`format` field).
    fn format(&self) -> Option<&DataTypeFormat>;

    /// Schema for array definitions, if any (`items` field).
    fn items(&self) -> Option<&Resolvable<Self>>;

    /// Mutable access to the `items` field, if it exists.
    fn items_mut(&mut self) -> Option<&mut Resolvable<Self>>;

    /// Value schema for maps (`additional_properties` field).
    fn additional_properties(&self) -> Option<&Either<bool, Resolvable<Self>>>;

    /// Mutable access to `additional_properties` field, if it's a map.
    fn additional_properties_mut(&mut self) -> Option<&mut Either<bool, Resolvable<Self>>>;

    /// Map of names and schema for properties, if it's an object (`properties` field)
    fn properties(&self) -> Option<&BTreeMap<String, Resolvable<Self>>>;

    /// Mutable access to `properties` field.
    fn properties_mut(&mut self) -> Option<&mut BTreeMap<String, Resolvable<Self>>>;

    /// Returns the required properties (if any) for this object.
    fn required_properties(&self) -> Option<&BTreeSet<String>>;

    /// Enum variants in this schema (if any). It's `serde_json::Value`
    /// because:
    ///
    /// - Enum variants are allowed to have any type of value.
    /// - `serde_json::Value` works for both JSON and YAML.
    fn enum_variants(&self) -> Option<&[serde_json::Value]>;

    /// Returns whether this definition "is" or "has" `Any` type.
    fn contains_any(&self) -> bool {
        _schema_contains_any(self, vec![])
    }

    /* MARK: Resolver-specific methods. */

    /// Set the reference to this schema.
    fn set_reference(&mut self, ref_: String);

    /// Set whether this definition is cyclic. This is done by the resolver.
    fn set_cyclic(&mut self, cyclic: bool);

    /// Returns whether this definition is cyclic.
    ///
    /// **NOTE:** This is not part of the schema object, but it's
    /// set by the resolver using `set_cyclic` for codegen.
    fn is_cyclic(&self) -> bool;

    /// Name of this schema, if any.
    ///
    /// **NOTE:** This is not part of the schema object, but it's
    /// set by the resolver using `set_name` for codegen.
    fn name(&self) -> Option<&str>;

    /// Sets the name for this schema. This is done by the resolver.
    fn set_name(&mut self, name: &str);
}

fn _schema_contains_any<'a, S: Schema>(schema: &'a S, mut nodes: Vec<&'a str>) -> bool {
    if schema.data_type().is_none() {
        return true;
    }

    if let Some(name) = schema.name() {
        if nodes.contains(&name) {
            return false; // We've encountered a cycle.
        } else {
            nodes.push(name);
        }
    }

    schema
        .properties()
        .map(|t| {
            t.values()
                .any(|s| _schema_contains_any(&*s.read().unwrap(), nodes.clone()))
        })
        .unwrap_or(false)
        || schema
            .items()
            .map(|s| _schema_contains_any(&*s.read().unwrap(), nodes.clone()))
            .unwrap_or(false)
        || schema
            .additional_properties()
            .map(|e| match e {
                Either::Left(extra_props_allowed) => *extra_props_allowed,
                Either::Right(s) => _schema_contains_any(&*s.read().unwrap(), nodes),
            })
            .unwrap_or(false)
}

/// Trait for returning OpenAPI data type and format for the implementor.
pub trait TypedData {
    /// The OpenAPI type for this implementor.
    fn data_type() -> DataType {
        DataType::Object
    }

    /// The optional OpenAPI data format for this implementor.
    fn format() -> Option<DataTypeFormat> {
        None
    }

    fn max() -> Option<f32> {
        None
    }

    fn min() -> Option<f32> {
        None
    }
}

macro_rules! impl_type_simple {
    ($ty:ty) => {
        impl TypedData for $ty {}
    };
    ($ty:ty, $dt:expr) => {
        impl TypedData for $ty {
            fn data_type() -> DataType {
                $dt
            }
        }
    };
    ($ty:ty, $dt:expr, $df:expr) => {
        impl TypedData for $ty {
            fn data_type() -> DataType {
                $dt
            }
            fn format() -> Option<DataTypeFormat> {
                Some($df)
            }
        }
    };
    ($ty:ty, $dt:expr, $df:expr, $min:expr, $max:expr) => {
        impl TypedData for $ty {
            fn data_type() -> DataType {
                $dt
            }
            fn format() -> Option<DataTypeFormat> {
                Some($df)
            }
            fn max() -> Option<f32> {
                Some($max)
            }
            fn min() -> Option<f32> {
                Some($min)
            }
        }
    };
}

impl TypedData for &str {
    fn data_type() -> DataType {
        DataType::String
    }
}

impl<T: TypedData> TypedData for &T {
    fn data_type() -> DataType {
        T::data_type()
    }

    fn format() -> Option<DataTypeFormat> {
        T::format()
    }
}

impl_type_simple!(char, DataType::String);
impl_type_simple!(String, DataType::String);
impl_type_simple!(PathBuf, DataType::String);
#[cfg(feature = "camino")]
impl_type_simple!(
    camino::Utf8PathBuf,
    DataType::String,
    DataTypeFormat::Binary
);
impl_type_simple!(bool, DataType::Boolean);
impl_type_simple!(f32, DataType::Number, DataTypeFormat::Float);
impl_type_simple!(f64, DataType::Number, DataTypeFormat::Double);
impl_type_simple!(
    i8,
    DataType::Integer,
    DataTypeFormat::Int32,
    i8::MIN as f32,
    i8::MAX as f32
);
impl_type_simple!(
    i16,
    DataType::Integer,
    DataTypeFormat::Int32,
    i16::MIN as f32,
    i16::MAX as f32
);
impl_type_simple!(i32, DataType::Integer, DataTypeFormat::Int32);
impl_type_simple!(
    u8,
    DataType::Integer,
    DataTypeFormat::Int32,
    u8::MIN as f32,
    u8::MAX as f32
);
impl_type_simple!(
    u16,
    DataType::Integer,
    DataTypeFormat::Int32,
    u16::MIN as f32,
    u16::MAX as f32
);
impl_type_simple!(u32, DataType::Integer, DataTypeFormat::Int32);
impl_type_simple!(i64, DataType::Integer, DataTypeFormat::Int64);
impl_type_simple!(
    i128,
    DataType::Integer,
    DataTypeFormat::Int64,
    i128::MIN as f32,
    i128::MAX as f32
);
impl_type_simple!(isize, DataType::Integer, DataTypeFormat::Int64);
impl_type_simple!(u64, DataType::Integer, DataTypeFormat::Int64);
impl_type_simple!(
    u128,
    DataType::Integer,
    DataTypeFormat::Int64,
    u128::MIN as f32,
    u128::MAX as f32
);
impl_type_simple!(usize, DataType::Integer, DataTypeFormat::Int64);

#[cfg(feature = "actix-multipart")]
impl_type_simple!(
    actix_multipart::Multipart,
    DataType::File,
    DataTypeFormat::Binary
);
#[cfg(feature = "actix-session")]
impl_type_simple!(actix_session::Session);
#[cfg(feature = "actix-identity")]
impl_type_simple!(actix_identity::Identity);
#[cfg(feature = "actix-files")]
impl_type_simple!(
    actix_files::NamedFile,
    DataType::File,
    DataTypeFormat::Binary
);
#[cfg(feature = "jiff01")]
impl_type_simple!(jiff::Timestamp, DataType::String, DataTypeFormat::DateTime);
#[cfg(feature = "jiff01")]
impl_type_simple!(
    jiff::Zoned,
    DataType::String,
    DataTypeFormat::Other //RFC 8536
);
#[cfg(feature = "jiff01")]
impl_type_simple!(
    jiff::civil::DateTime,
    DataType::String,
    DataTypeFormat::Other //2024-06-19T15:22:45
);
#[cfg(feature = "jiff01")]
impl_type_simple!(jiff::civil::Date, DataType::String, DataTypeFormat::Date);
#[cfg(feature = "jiff01")]
impl_type_simple!(
    jiff::civil::Time,
    DataType::String,
    DataTypeFormat::Other //15:22:45
);
#[cfg(feature = "jiff01")]
impl_type_simple!(
    jiff::Span,
    DataType::String,
    DataTypeFormat::Other //ISO 8601
);
#[cfg(feature = "chrono")]
impl_type_simple!(
    chrono::NaiveDateTime,
    DataType::String,
    DataTypeFormat::DateTime
);
#[cfg(feature = "chrono")]
impl_type_simple!(chrono::NaiveDate, DataType::String, DataTypeFormat::Date);
#[cfg(feature = "chrono")]
impl_type_simple!(chrono::NaiveTime, DataType::String);
#[cfg(feature = "rust_decimal")]
impl_type_simple!(
    rust_decimal::Decimal,
    DataType::Number,
    DataTypeFormat::Float
);

#[cfg(feature = "url")]
impl_type_simple!(url::Url, DataType::String, DataTypeFormat::Url);

#[cfg(feature = "uuid0")]
impl_type_simple!(uuid0_dep::Uuid, DataType::String, DataTypeFormat::Uuid);
#[cfg(feature = "uuid1")]
impl_type_simple!(uuid1_dep::Uuid, DataType::String, DataTypeFormat::Uuid);

#[cfg(feature = "chrono")]
impl<T: chrono::offset::TimeZone> TypedData for chrono::DateTime<T> {
    fn data_type() -> DataType {
        DataType::String
    }
    fn format() -> Option<DataTypeFormat> {
        Some(DataTypeFormat::DateTime)
    }
}

#[cfg(feature = "chrono")]
#[allow(deprecated)]
impl<T: chrono::offset::TimeZone> TypedData for chrono::Date<T> {
    fn data_type() -> DataType {
        DataType::String
    }
    fn format() -> Option<DataTypeFormat> {
        Some(DataTypeFormat::Date)
    }
}

impl_type_simple!(std::net::IpAddr, DataType::String, DataTypeFormat::Ip);
impl_type_simple!(std::net::Ipv4Addr, DataType::String, DataTypeFormat::IpV4);
impl_type_simple!(std::net::Ipv6Addr, DataType::String, DataTypeFormat::IpV6);

/// Represents a OpenAPI v2 schema object convertible. This is auto-implemented by
/// framework-specific macros:
///
/// - [`Apiv2Schema`](https://paperclip-rs.github.io/paperclip/paperclip_actix/derive.Apiv2Schema.html)
///   for schema objects.
/// - [`Apiv2Security`](https://paperclip-rs.github.io/paperclip/paperclip_actix/derive.Apiv2Security.html)
///   for security scheme objects.
/// - [`Apiv2Header`](https://paperclip-rs.github.io/paperclip/paperclip_actix/derive.Apiv2Header.html)
///   for header parameters objects.
///
/// This is implemented for primitive types by default.
pub trait Apiv2Schema {
    /// Name of this schema. This is the name to which the definition of the object is mapped.
    fn name() -> Option<String> {
        None
    }

    /// Description of this schema. In case the trait is derived, uses the documentation on the type.
    fn description() -> &'static str {
        ""
    }

    /// Indicates the requirement of this schema.
    fn required() -> bool {
        true
    }

    /// Returns the raw schema for this object.
    fn raw_schema() -> DefaultSchemaRaw {
        Default::default()
    }

    /// Returns the schema with a reference (if this is an object).
    ///
    /// Here, we set the global reference to this object using its name,
    /// and we either remove the reference (`remove_refs`) or remove all
    /// properties other than the reference (`retain_ref`) based on where
    /// we're storing this object in the spec i.e., in an operation/response
    /// or in the map of definitions.
    ///
    /// And we do that because at the time of this writing, statically
    /// collecting all models for all operations involved a lot of work,
    /// and so I went for runtime collection. Even though this happens at
    /// runtime, we're only doing this once at the start of the application,
    /// so it won't affect the incoming requests at all.
    fn schema_with_ref() -> DefaultSchemaRaw {
        let mut def = Self::raw_schema();
        if let Some(n) = Self::name() {
            def.reference =
                Some(String::from("#/definitions/") + &n.replace('<', "%3C").replace('>', "%3E"));
        } else if let Some(n) = def.name.as_ref() {
            def.reference =
                Some(String::from("#/definitions/") + &n.replace('<', "%3C").replace('>', "%3E"));
        }
        if !Self::description().is_empty() {
            def.description = Some(Self::description().to_owned());
        }

        def
    }

    /// Returns the security scheme for this object.
    fn security_scheme() -> Option<SecurityScheme> {
        None
    }

    fn header_parameter_schema() -> Vec<Parameter<DefaultSchemaRaw>> {
        vec![]
    }
}

impl Apiv2Schema for () {}
impl Apiv2Schema for serde_json::Value {}
impl Apiv2Schema for serde_yaml::Value {}

impl<T: TypedData> Apiv2Schema for T {
    fn raw_schema() -> DefaultSchemaRaw {
        DefaultSchemaRaw {
            data_type: Some(T::data_type()),
            format: T::format(),
            maximum: T::max(),
            minimum: T::min(),
            ..Default::default()
        }
    }
}

#[cfg(feature = "nightly")]
impl<T> Apiv2Schema for Option<T> {
    default fn name() -> Option<String> {
        None
    }

    default fn required() -> bool {
        false
    }

    default fn raw_schema() -> DefaultSchemaRaw {
        Default::default()
    }

    default fn security_scheme() -> Option<SecurityScheme> {
        None
    }

    default fn header_parameter_schema() -> Vec<Parameter<DefaultSchemaRaw>> {
        vec![]
    }
}

impl<T: Apiv2Schema> Apiv2Schema for Option<T> {
    fn name() -> Option<String> {
        T::name()
    }

    fn required() -> bool {
        false
    }

    fn raw_schema() -> DefaultSchemaRaw {
        T::raw_schema()
    }

    fn security_scheme() -> Option<SecurityScheme> {
        T::security_scheme()
    }

    fn header_parameter_schema() -> Vec<Parameter<DefaultSchemaRaw>> {
        T::header_parameter_schema()
    }
}

#[cfg(feature = "nightly")]
impl<T, E> Apiv2Schema for Result<T, E> {
    default fn name() -> Option<String> {
        None
    }

    default fn raw_schema() -> DefaultSchemaRaw {
        Default::default()
    }

    default fn security_scheme() -> Option<SecurityScheme> {
        Default::default()
    }

    default fn header_parameter_schema() -> Vec<Parameter<DefaultSchemaRaw>> {
        Default::default()
    }
}

impl<T: Apiv2Schema, E> Apiv2Schema for Result<T, E> {
    fn name() -> Option<String> {
        T::name()
    }

    fn raw_schema() -> DefaultSchemaRaw {
        T::raw_schema()
    }

    fn security_scheme() -> Option<SecurityScheme> {
        T::security_scheme()
    }

    fn header_parameter_schema() -> Vec<Parameter<DefaultSchemaRaw>> {
        T::header_parameter_schema()
    }
}

impl<T: Apiv2Schema + Clone> Apiv2Schema for std::borrow::Cow<'_, T> {
    fn name() -> Option<String> {
        T::name()
    }

    fn raw_schema() -> DefaultSchemaRaw {
        T::raw_schema()
    }

    fn security_scheme() -> Option<SecurityScheme> {
        T::security_scheme()
    }

    fn header_parameter_schema() -> Vec<Parameter<DefaultSchemaRaw>> {
        T::header_parameter_schema()
    }
}

impl<T: Apiv2Schema> Apiv2Schema for &[T] {
    fn raw_schema() -> DefaultSchemaRaw {
        Vec::<T>::raw_schema()
    }
}

impl<T: Apiv2Schema, const N: usize> Apiv2Schema for [T; N] {
    fn raw_schema() -> DefaultSchemaRaw {
        DefaultSchemaRaw {
            data_type: Some(DataType::Array),
            items: Some(T::schema_with_ref().into()),
            ..Default::default()
        }
    }
}

macro_rules! impl_schema_array {
    ($ty:ty) => {
        impl<T: Apiv2Schema> Apiv2Schema for $ty {
            fn raw_schema() -> DefaultSchemaRaw {
                DefaultSchemaRaw {
                    data_type: Some(DataType::Array),
                    items: Some(T::schema_with_ref().into()),
                    ..Default::default()
                }
            }
        }
    };
}

macro_rules! impl_schema_map {
    ($ty:ty) => {
        impl<K: ToString, V: Apiv2Schema> Apiv2Schema for $ty {
            fn raw_schema() -> DefaultSchemaRaw {
                DefaultSchemaRaw {
                    data_type: Some(DataType::Object),
                    extra_props: Some(Either::Right(V::schema_with_ref().into())),
                    ..Default::default()
                }
            }
        }
    };
}

use crate::v2::models::Parameter;
use std::{collections::*, path::PathBuf};

impl_schema_array!(Vec<T>);
impl_schema_array!(HashSet<T>);
impl_schema_array!(LinkedList<T>);
impl_schema_array!(VecDeque<T>);
impl_schema_array!(BTreeSet<T>);
impl_schema_array!(BinaryHeap<T>);

impl_schema_map!(HashMap<K, V>);
impl_schema_map!(BTreeMap<K, V>);

/// Represents a OpenAPI v2 operation convertible. This is auto-implemented by
/// framework-specific macros:
///
/// - [`paperclip_actix::api_v2_operation`](https://paperclip-rs.github.io/paperclip/paperclip_actix/attr.api_v2_operation.html).
///
/// **NOTE:** The type parameters specified here aren't used by the trait itself,
/// but *can* be used for constraining stuff in framework-related impls.
pub trait Apiv2Operation {
    /// Returns the definition for this operation.
    fn operation() -> DefaultOperationRaw;

    /// Returns a map of security definitions that will be merged globally.
    fn security_definitions() -> BTreeMap<String, SecurityScheme>;

    /// Returns the definitions used by this operation.
    fn definitions() -> BTreeMap<String, DefaultSchemaRaw>;

    fn is_visible() -> bool {
        true
    }
}

/// Represents a OpenAPI v2 error convertible. This is auto-implemented by
/// framework-specific macros:
///
/// - [`paperclip_actix::api_v2_errors`](https://paperclip-rs.github.io/paperclip/paperclip_actix/attr.api_v2_errors.html).
pub trait Apiv2Errors {
    const ERROR_MAP: &'static [(u16, &'static str)] = &[];
    fn update_error_definitions(_op: &mut DefaultOperationRaw) {}
    fn update_definitions(_map: &mut BTreeMap<String, DefaultSchemaRaw>) {}
}

impl Apiv2Errors for () {}
#[cfg(feature = "actix-base")]
impl Apiv2Errors for actix_web::Error {}
