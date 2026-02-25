use pyo3::prelude::*;
use pyo3::types::{PyDict, PyFloat, PyList, PyString};
use serde::Serialize;
use serde_json::Value;

/// Convert a serde_json::Value to a Python object.
fn value_to_py(py: Python<'_>, value: &Value) -> PyResult<PyObject> {
    match value {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => Ok(b.into_pyobject(py)?.to_owned().into_any().unbind()),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_pyobject(py)?.into_any().unbind())
            } else if let Some(f) = n.as_f64() {
                Ok(PyFloat::new(py, f).into_any().unbind())
            } else {
                Ok(py.None())
            }
        }
        Value::String(s) => Ok(PyString::new(py, s).into_any().unbind()),
        Value::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(value_to_py(py, item)?)?;
            }
            Ok(list.into_any().unbind())
        }
        Value::Object(map) => {
            let dict = PyDict::new(py);
            for (k, v) in map {
                dict.set_item(k, value_to_py(py, v)?)?;
            }
            Ok(dict.into_any().unbind())
        }
    }
}

/// Serialize a Rust value to a Python dict via serde_json.
pub fn rust_to_py_dict(py: Python<'_>, value: &impl Serialize) -> PyResult<PyObject> {
    let json_value = serde_json::to_value(value)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    value_to_py(py, &json_value)
}

/// Serialize a Rust value to a JSON string.
pub fn rust_to_json_string(value: &impl Serialize, indent: Option<usize>) -> PyResult<String> {
    match indent {
        Some(n) => {
            let buf = Vec::new();
            let indent_str = b" ".repeat(n);
            let formatter = serde_json::ser::PrettyFormatter::with_indent(&indent_str);
            let mut ser = serde_json::Serializer::with_formatter(buf, formatter);
            value
                .serialize(&mut ser)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            String::from_utf8(ser.into_inner())
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
        }
        None => serde_json::to_string(value)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string())),
    }
}
