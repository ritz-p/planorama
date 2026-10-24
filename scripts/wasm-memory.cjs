function memoryLimits(bytes) {
  let offset = 8;
  const integer = () => {
    let value = 0, scale = 1, byte;
    do {
      if (offset >= bytes.length || scale > 2 ** 28) throw new Error('Invalid Wasm integer');
      byte = bytes[offset++];
      value += (byte & 127) * scale;
      scale *= 128;
    } while (byte & 128);
    return value;
  };
  const name = () => { const length = integer(); offset += length; };
  const limits = () => {
    const flags = integer(), minimum = integer();
    if (flags !== 1) throw new Error('Expected unshared wasm32 memory with maximum');
    return {minimum, maximum: integer()};
  };
  const memories = [];
  while (offset < bytes.length) {
    const section = bytes[offset++], length = integer(), end = offset + length;
    if (section === 2) {
      const count = integer();
      for (let i = 0; i < count; i++) {
        name(); name();
        switch (bytes[offset++]) {
          case 0: integer(); break;
          case 1: {
            offset++;
            const flags = integer(); integer();
            if (flags & 1) integer();
            break;
          }
          case 2: memories.push(limits()); break;
          case 3: offset += 2; break;
          case 4: integer(); integer(); break;
          default: throw new Error('Unsupported Wasm import');
        }
      }
    }
    if (section === 5 && integer() !== 0) throw new Error('Unexpected defined memory');
    offset = end;
  }
  if (memories.length !== 1) throw new Error('Expected one imported memory');
  return memories[0];
}

module.exports = {memoryLimits};
