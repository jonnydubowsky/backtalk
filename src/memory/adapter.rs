use futures::future::{Future, BoxFuture, ok, err};
use {JsonValue, ErrorKind, Adapter, JsonObject};
use std::collections::HashMap;
use std::sync::Mutex;

fn std_error(kind: ErrorKind, err_str: &str) -> (ErrorKind, JsonValue) {
  let val = json!({
    "error": {
      "type": kind.as_string(),
      "message": err_str.to_string(),
    }
  });
  (kind, val)
}

pub struct MemoryAdapter {
  inside: Mutex<Inside>,
}

struct Inside {
  datastore: HashMap<String, JsonObject>,
  last_num: i64,
}

impl MemoryAdapter {
  pub fn new() -> MemoryAdapter {
    MemoryAdapter {
      inside: Mutex::new(Inside {
        datastore: HashMap::new(),
        last_num: 0,
      }),
    }
  }
}

impl Adapter for MemoryAdapter {
  /// currently this function only supports equality matching — we'd probably want to add more
  /// kinds of matching and querying in the future, maybe by building into query object
  fn list(&self, params: &JsonObject) -> BoxFuture<JsonObject, (ErrorKind, JsonValue)> {
    let inside = self.inside.lock().unwrap();
    let res: Vec<JsonValue> = inside.datastore
      .iter()
      .map(|(_, item)| item)
      .filter(|item| {
        for (param_key, param_val) in params {
          if item.get(param_key) != Some(param_val) {
            return false;
          }
        }
        true
      })
      .map(|item| JsonValue::Object(item.clone()))
      .collect();
    let mut dat = JsonObject::new();
    dat.insert("data".to_string(), JsonValue::Array(res));
    ok(dat).boxed()
  }

  fn get(&self, id: &str, _params: &JsonObject) -> BoxFuture<JsonObject, (ErrorKind, JsonValue)> {
    let inside = self.inside.lock().unwrap();
    match inside.datastore.get(id) {
      Some(val) => ok(val.clone()).boxed(),
      None => err(std_error(ErrorKind::NotFound, "couldn't find object with that id")).boxed(),
    }
  }

  fn post(&self, data: &JsonObject, _params: &JsonObject) -> BoxFuture<JsonObject, (ErrorKind, JsonValue)> {
    let mut inside = self.inside.lock().unwrap();
    inside.last_num += 1;
    let mut data = data.clone(); // TODO remove clones?
    let id_str = inside.last_num.to_string();
    data.insert("id".to_string(), JsonValue::String(id_str.clone()));
    inside.datastore.insert(id_str, data.clone());
    ok(data).boxed()
  }

  fn patch(&self, id: &str, data: &JsonObject, _params: &JsonObject) -> BoxFuture<JsonObject, (ErrorKind, JsonValue)> {
    let mut inside = self.inside.lock().unwrap();
    if let Some(_) = data.get("id") {
      return err(std_error(ErrorKind::BadRequest, "can't update id")).boxed();
    }
    let dbdata = match inside.datastore.get_mut(id) {
      Some(val) => val,
      None => return err(std_error(ErrorKind::NotFound, "couldn't find object with that id")).boxed(),
    };
    // TODO should probably recursively update children too instead of replacing, there's a JSON update spec that you can read
    for (key, val) in data.clone().into_iter() {
      dbdata.insert(key, val);
    }
    ok(dbdata.clone()).boxed()
  }

  fn delete(&self, id: &str, _params: &JsonObject) -> BoxFuture<JsonObject, (ErrorKind, JsonValue)> {
    let mut inside = self.inside.lock().unwrap();
    inside.datastore.remove(id);
    let mut data = JsonObject::new();
    data.insert("id".to_string(), JsonValue::String(id.to_string()));
    ok(data).boxed()
  }
}
