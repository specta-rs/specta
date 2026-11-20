// A deliberately small JSON validator. It rejects unknown fields and every
// wrong type, because the output of a model is untrusted input to this program.

export function validate(value, spec, path = "$") {
  const errors = [];
  walk(value, spec, path, errors);
  return errors;
}

function walk(value, spec, path, errors) {
  if (value === null || value === undefined) {
    if (spec.nullable) return;
    errors.push(`${path}: expected ${spec.type}, got ${value === null ? "null" : "undefined"}`);
    return;
  }

  switch (spec.type) {
    case "string": {
      if (typeof value !== "string") return errors.push(`${path}: expected string`);
      if (spec.max !== undefined && value.length > spec.max) {
        errors.push(`${path}: ${value.length} characters, limit is ${spec.max}`);
      }
      if (spec.pattern && !spec.pattern.test(value)) errors.push(`${path}: does not match ${spec.pattern}`);
      return;
    }
    case "integer": {
      if (!Number.isInteger(value)) return errors.push(`${path}: expected integer`);
      if (spec.min !== undefined && value < spec.min) errors.push(`${path}: must be >= ${spec.min}`);
      if (spec.max !== undefined && value > spec.max) errors.push(`${path}: must be <= ${spec.max}`);
      return;
    }
    case "boolean": {
      if (typeof value !== "boolean") errors.push(`${path}: expected boolean`);
      return;
    }
    case "enum": {
      if (!spec.values.includes(value)) errors.push(`${path}: expected one of ${spec.values.join(", ")}`);
      return;
    }
    case "array": {
      if (!Array.isArray(value)) return errors.push(`${path}: expected array`);
      if (spec.max !== undefined && value.length > spec.max) {
        errors.push(`${path}: ${value.length} items, limit is ${spec.max}`);
      }
      value.forEach((item, index) => walk(item, spec.of, `${path}[${index}]`, errors));
      return;
    }
    case "object": {
      if (typeof value !== "object" || Array.isArray(value)) return errors.push(`${path}: expected object`);
      for (const key of Object.keys(value)) {
        if (!(key in spec.fields)) errors.push(`${path}.${key}: unexpected field`);
      }
      for (const [key, fieldSpec] of Object.entries(spec.fields)) {
        if (!(key in value)) {
          if (fieldSpec.required !== false) errors.push(`${path}.${key}: required`);
          continue;
        }
        walk(value[key], fieldSpec, `${path}.${key}`, errors);
      }
      return;
    }
    default:
      errors.push(`${path}: bad schema node ${spec.type}`);
  }
}

export function parseJsonLoose(text) {
  const raw = String(text ?? "").trim();
  const unfenced = raw.replace(/^\s*```(?:json)?\s*/i, "").replace(/\s*```\s*$/, "").trim();
  const first = unfenced.indexOf("{");
  const last = unfenced.lastIndexOf("}");
  if (first < 0 || last <= first) throw new Error("response did not contain a JSON object");
  return JSON.parse(unfenced.slice(first, last + 1));
}
