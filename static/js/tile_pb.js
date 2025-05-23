// source: code/tobmap/crates/tilebuildvector/proto/tile.proto
/**
 * @fileoverview
 * @enhanceable
 * @suppress {missingRequire} reports error on implicit type usages.
 * @suppress {messageConventions} JS Compiler reports an error if a variable or
 *     field starts with 'MSG_' and isn't a translatable message.
 * @public
 */
// GENERATED CODE -- DO NOT EDIT!
/* eslint-disable */
// @ts-nocheck

var jspb = require('google-protobuf');
var goog = jspb;
var global =
    (typeof globalThis !== 'undefined' && globalThis) ||
    (typeof window !== 'undefined' && window) ||
    (typeof global !== 'undefined' && global) ||
    (typeof self !== 'undefined' && self) ||
    (function () { return this; }).call(null) ||
    Function('return this')();

goog.exportSymbol('proto.tobmapdata.Edge', null, global);
goog.exportSymbol('proto.tobmapdata.S2CellData', null, global);
goog.exportSymbol('proto.tobmapdata.Vertex', null, global);
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.tobmapdata.S2CellData = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.tobmapdata.S2CellData.repeatedFields_, null);
};
goog.inherits(proto.tobmapdata.S2CellData, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.tobmapdata.S2CellData.displayName = 'proto.tobmapdata.S2CellData';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.tobmapdata.Vertex = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, null, null);
};
goog.inherits(proto.tobmapdata.Vertex, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.tobmapdata.Vertex.displayName = 'proto.tobmapdata.Vertex';
}
/**
 * Generated by JsPbCodeGenerator.
 * @param {Array=} opt_data Optional initial data array, typically from a
 * server response, or constructed directly in Javascript. The array is used
 * in place and becomes part of the constructed object. It is not cloned.
 * If no data is provided, the constructed object will be empty, but still
 * valid.
 * @extends {jspb.Message}
 * @constructor
 */
proto.tobmapdata.Edge = function(opt_data) {
  jspb.Message.initialize(this, opt_data, 0, -1, proto.tobmapdata.Edge.repeatedFields_, null);
};
goog.inherits(proto.tobmapdata.Edge, jspb.Message);
if (goog.DEBUG && !COMPILED) {
  /**
   * @public
   * @override
   */
  proto.tobmapdata.Edge.displayName = 'proto.tobmapdata.Edge';
}

/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.tobmapdata.S2CellData.repeatedFields_ = [2,3];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.tobmapdata.S2CellData.prototype.toObject = function(opt_includeInstance) {
  return proto.tobmapdata.S2CellData.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.tobmapdata.S2CellData} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.tobmapdata.S2CellData.toObject = function(includeInstance, msg) {
  var f, obj = {
cellId: jspb.Message.getFieldWithDefault(msg, 1, 0),
verticesList: jspb.Message.toObjectList(msg.getVerticesList(),
    proto.tobmapdata.Vertex.toObject, includeInstance),
edgesList: jspb.Message.toObjectList(msg.getEdgesList(),
    proto.tobmapdata.Edge.toObject, includeInstance)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.tobmapdata.S2CellData}
 */
proto.tobmapdata.S2CellData.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.tobmapdata.S2CellData;
  return proto.tobmapdata.S2CellData.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.tobmapdata.S2CellData} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.tobmapdata.S2CellData}
 */
proto.tobmapdata.S2CellData.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {number} */ (reader.readUint64());
      msg.setCellId(value);
      break;
    case 2:
      var value = new proto.tobmapdata.Vertex;
      reader.readMessage(value,proto.tobmapdata.Vertex.deserializeBinaryFromReader);
      msg.addVertices(value);
      break;
    case 3:
      var value = new proto.tobmapdata.Edge;
      reader.readMessage(value,proto.tobmapdata.Edge.deserializeBinaryFromReader);
      msg.addEdges(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.tobmapdata.S2CellData.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.tobmapdata.S2CellData.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.tobmapdata.S2CellData} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.tobmapdata.S2CellData.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getCellId();
  if (f !== 0) {
    writer.writeUint64(
      1,
      f
    );
  }
  f = message.getVerticesList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      2,
      f,
      proto.tobmapdata.Vertex.serializeBinaryToWriter
    );
  }
  f = message.getEdgesList();
  if (f.length > 0) {
    writer.writeRepeatedMessage(
      3,
      f,
      proto.tobmapdata.Edge.serializeBinaryToWriter
    );
  }
};


/**
 * optional uint64 cell_id = 1;
 * @return {number}
 */
proto.tobmapdata.S2CellData.prototype.getCellId = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 1, 0));
};


/**
 * @param {number} value
 * @return {!proto.tobmapdata.S2CellData} returns this
 */
proto.tobmapdata.S2CellData.prototype.setCellId = function(value) {
  return jspb.Message.setProto3IntField(this, 1, value);
};


/**
 * repeated Vertex vertices = 2;
 * @return {!Array<!proto.tobmapdata.Vertex>}
 */
proto.tobmapdata.S2CellData.prototype.getVerticesList = function() {
  return /** @type{!Array<!proto.tobmapdata.Vertex>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.tobmapdata.Vertex, 2));
};


/**
 * @param {!Array<!proto.tobmapdata.Vertex>} value
 * @return {!proto.tobmapdata.S2CellData} returns this
*/
proto.tobmapdata.S2CellData.prototype.setVerticesList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 2, value);
};


/**
 * @param {!proto.tobmapdata.Vertex=} opt_value
 * @param {number=} opt_index
 * @return {!proto.tobmapdata.Vertex}
 */
proto.tobmapdata.S2CellData.prototype.addVertices = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 2, opt_value, proto.tobmapdata.Vertex, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.tobmapdata.S2CellData} returns this
 */
proto.tobmapdata.S2CellData.prototype.clearVerticesList = function() {
  return this.setVerticesList([]);
};


/**
 * repeated Edge edges = 3;
 * @return {!Array<!proto.tobmapdata.Edge>}
 */
proto.tobmapdata.S2CellData.prototype.getEdgesList = function() {
  return /** @type{!Array<!proto.tobmapdata.Edge>} */ (
    jspb.Message.getRepeatedWrapperField(this, proto.tobmapdata.Edge, 3));
};


/**
 * @param {!Array<!proto.tobmapdata.Edge>} value
 * @return {!proto.tobmapdata.S2CellData} returns this
*/
proto.tobmapdata.S2CellData.prototype.setEdgesList = function(value) {
  return jspb.Message.setRepeatedWrapperField(this, 3, value);
};


/**
 * @param {!proto.tobmapdata.Edge=} opt_value
 * @param {number=} opt_index
 * @return {!proto.tobmapdata.Edge}
 */
proto.tobmapdata.S2CellData.prototype.addEdges = function(opt_value, opt_index) {
  return jspb.Message.addToRepeatedWrapperField(this, 3, opt_value, proto.tobmapdata.Edge, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.tobmapdata.S2CellData} returns this
 */
proto.tobmapdata.S2CellData.prototype.clearEdgesList = function() {
  return this.setEdgesList([]);
};





if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.tobmapdata.Vertex.prototype.toObject = function(opt_includeInstance) {
  return proto.tobmapdata.Vertex.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.tobmapdata.Vertex} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.tobmapdata.Vertex.toObject = function(includeInstance, msg) {
  var f, obj = {
cellId: jspb.Message.getFieldWithDefault(msg, 1, 0)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.tobmapdata.Vertex}
 */
proto.tobmapdata.Vertex.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.tobmapdata.Vertex;
  return proto.tobmapdata.Vertex.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.tobmapdata.Vertex} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.tobmapdata.Vertex}
 */
proto.tobmapdata.Vertex.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var value = /** @type {number} */ (reader.readUint64());
      msg.setCellId(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.tobmapdata.Vertex.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.tobmapdata.Vertex.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.tobmapdata.Vertex} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.tobmapdata.Vertex.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getCellId();
  if (f !== 0) {
    writer.writeUint64(
      1,
      f
    );
  }
};


/**
 * optional uint64 cell_id = 1;
 * @return {number}
 */
proto.tobmapdata.Vertex.prototype.getCellId = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 1, 0));
};


/**
 * @param {number} value
 * @return {!proto.tobmapdata.Vertex} returns this
 */
proto.tobmapdata.Vertex.prototype.setCellId = function(value) {
  return jspb.Message.setProto3IntField(this, 1, value);
};



/**
 * List of repeated fields within this message type.
 * @private {!Array<number>}
 * @const
 */
proto.tobmapdata.Edge.repeatedFields_ = [1,3];



if (jspb.Message.GENERATE_TO_OBJECT) {
/**
 * Creates an object representation of this proto.
 * Field names that are reserved in JavaScript and will be renamed to pb_name.
 * Optional fields that are not set will be set to undefined.
 * To access a reserved field use, foo.pb_<name>, eg, foo.pb_default.
 * For the list of reserved names please see:
 *     net/proto2/compiler/js/internal/generator.cc#kKeyword.
 * @param {boolean=} opt_includeInstance Deprecated. whether to include the
 *     JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @return {!Object}
 */
proto.tobmapdata.Edge.prototype.toObject = function(opt_includeInstance) {
  return proto.tobmapdata.Edge.toObject(opt_includeInstance, this);
};


/**
 * Static version of the {@see toObject} method.
 * @param {boolean|undefined} includeInstance Deprecated. Whether to include
 *     the JSPB instance for transitional soy proto support:
 *     http://goto/soy-param-migration
 * @param {!proto.tobmapdata.Edge} msg The msg instance to transform.
 * @return {!Object}
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.tobmapdata.Edge.toObject = function(includeInstance, msg) {
  var f, obj = {
pointsList: (f = jspb.Message.getRepeatedField(msg, 1)) == null ? undefined : f,
priority: jspb.Message.getFieldWithDefault(msg, 2, 0),
streetNamesList: (f = jspb.Message.getRepeatedField(msg, 3)) == null ? undefined : f,
isOneway: jspb.Message.getBooleanFieldWithDefault(msg, 4, false)
  };

  if (includeInstance) {
    obj.$jspbMessageInstance = msg;
  }
  return obj;
};
}


/**
 * Deserializes binary data (in protobuf wire format).
 * @param {jspb.ByteSource} bytes The bytes to deserialize.
 * @return {!proto.tobmapdata.Edge}
 */
proto.tobmapdata.Edge.deserializeBinary = function(bytes) {
  var reader = new jspb.BinaryReader(bytes);
  var msg = new proto.tobmapdata.Edge;
  return proto.tobmapdata.Edge.deserializeBinaryFromReader(msg, reader);
};


/**
 * Deserializes binary data (in protobuf wire format) from the
 * given reader into the given message object.
 * @param {!proto.tobmapdata.Edge} msg The message object to deserialize into.
 * @param {!jspb.BinaryReader} reader The BinaryReader to use.
 * @return {!proto.tobmapdata.Edge}
 */
proto.tobmapdata.Edge.deserializeBinaryFromReader = function(msg, reader) {
  while (reader.nextField()) {
    if (reader.isEndGroup()) {
      break;
    }
    var field = reader.getFieldNumber();
    switch (field) {
    case 1:
      var values = /** @type {!Array<number>} */ (reader.isDelimited() ? reader.readPackedUint64() : [reader.readUint64()]);
      for (var i = 0; i < values.length; i++) {
        msg.addPoints(values[i]);
      }
      break;
    case 2:
      var value = /** @type {number} */ (reader.readUint32());
      msg.setPriority(value);
      break;
    case 3:
      var value = /** @type {string} */ (reader.readString());
      msg.addStreetNames(value);
      break;
    case 4:
      var value = /** @type {boolean} */ (reader.readBool());
      msg.setIsOneway(value);
      break;
    default:
      reader.skipField();
      break;
    }
  }
  return msg;
};


/**
 * Serializes the message to binary data (in protobuf wire format).
 * @return {!Uint8Array}
 */
proto.tobmapdata.Edge.prototype.serializeBinary = function() {
  var writer = new jspb.BinaryWriter();
  proto.tobmapdata.Edge.serializeBinaryToWriter(this, writer);
  return writer.getResultBuffer();
};


/**
 * Serializes the given message to binary data (in protobuf wire
 * format), writing to the given BinaryWriter.
 * @param {!proto.tobmapdata.Edge} message
 * @param {!jspb.BinaryWriter} writer
 * @suppress {unusedLocalVariables} f is only used for nested messages
 */
proto.tobmapdata.Edge.serializeBinaryToWriter = function(message, writer) {
  var f = undefined;
  f = message.getPointsList();
  if (f.length > 0) {
    writer.writePackedUint64(
      1,
      f
    );
  }
  f = message.getPriority();
  if (f !== 0) {
    writer.writeUint32(
      2,
      f
    );
  }
  f = message.getStreetNamesList();
  if (f.length > 0) {
    writer.writeRepeatedString(
      3,
      f
    );
  }
  f = message.getIsOneway();
  if (f) {
    writer.writeBool(
      4,
      f
    );
  }
};


/**
 * repeated uint64 points = 1;
 * @return {!Array<number>}
 */
proto.tobmapdata.Edge.prototype.getPointsList = function() {
  return /** @type {!Array<number>} */ (jspb.Message.getRepeatedField(this, 1));
};


/**
 * @param {!Array<number>} value
 * @return {!proto.tobmapdata.Edge} returns this
 */
proto.tobmapdata.Edge.prototype.setPointsList = function(value) {
  return jspb.Message.setField(this, 1, value || []);
};


/**
 * @param {number} value
 * @param {number=} opt_index
 * @return {!proto.tobmapdata.Edge} returns this
 */
proto.tobmapdata.Edge.prototype.addPoints = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 1, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.tobmapdata.Edge} returns this
 */
proto.tobmapdata.Edge.prototype.clearPointsList = function() {
  return this.setPointsList([]);
};


/**
 * optional uint32 priority = 2;
 * @return {number}
 */
proto.tobmapdata.Edge.prototype.getPriority = function() {
  return /** @type {number} */ (jspb.Message.getFieldWithDefault(this, 2, 0));
};


/**
 * @param {number} value
 * @return {!proto.tobmapdata.Edge} returns this
 */
proto.tobmapdata.Edge.prototype.setPriority = function(value) {
  return jspb.Message.setProto3IntField(this, 2, value);
};


/**
 * repeated string street_names = 3;
 * @return {!Array<string>}
 */
proto.tobmapdata.Edge.prototype.getStreetNamesList = function() {
  return /** @type {!Array<string>} */ (jspb.Message.getRepeatedField(this, 3));
};


/**
 * @param {!Array<string>} value
 * @return {!proto.tobmapdata.Edge} returns this
 */
proto.tobmapdata.Edge.prototype.setStreetNamesList = function(value) {
  return jspb.Message.setField(this, 3, value || []);
};


/**
 * @param {string} value
 * @param {number=} opt_index
 * @return {!proto.tobmapdata.Edge} returns this
 */
proto.tobmapdata.Edge.prototype.addStreetNames = function(value, opt_index) {
  return jspb.Message.addToRepeatedField(this, 3, value, opt_index);
};


/**
 * Clears the list making it empty but non-null.
 * @return {!proto.tobmapdata.Edge} returns this
 */
proto.tobmapdata.Edge.prototype.clearStreetNamesList = function() {
  return this.setStreetNamesList([]);
};


/**
 * optional bool is_oneway = 4;
 * @return {boolean}
 */
proto.tobmapdata.Edge.prototype.getIsOneway = function() {
  return /** @type {boolean} */ (jspb.Message.getBooleanFieldWithDefault(this, 4, false));
};


/**
 * @param {boolean} value
 * @return {!proto.tobmapdata.Edge} returns this
 */
proto.tobmapdata.Edge.prototype.setIsOneway = function(value) {
  return jspb.Message.setProto3BooleanField(this, 4, value);
};


goog.object.extend(exports, proto.tobmapdata);
