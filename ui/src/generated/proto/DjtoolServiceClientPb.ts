/**
 * @fileoverview gRPC-Web generated client stub for proto.djtool
 * @enhanceable
 * @public
 */

// GENERATED CODE -- DO NOT EDIT!


/* eslint-disable */
// @ts-nocheck


import * as grpcWeb from 'grpc-web';

import {
  ConnectRequest,
  DisconnectRequest,
  Empty,
  Update} from './djtool_pb';

export class DjtoolClient {
  client_: grpcWeb.AbstractClientBase;
  hostname_: string;
  credentials_: null | { [index: string]: string; };
  options_: null | { [index: string]: string; };

  constructor (hostname: string,
               credentials?: null | { [index: string]: string; },
               options?: null | { [index: string]: string; }) {
    if (!options) options = {};
    if (!credentials) credentials = {};
    options['format'] = 'text';

    this.client_ = new grpcWeb.GrpcWebClientBase(options);
    this.hostname_ = hostname;
    this.credentials_ = credentials;
    this.options_ = options;
  }

  methodInfoConnect = new grpcWeb.AbstractClientBase.MethodInfo(
    Update,
    (request: ConnectRequest) => {
      return request.serializeBinary();
    },
    Update.deserializeBinary
  );

  connect(
    request: ConnectRequest,
    metadata?: grpcWeb.Metadata) {
    return this.client_.serverStreaming(
      new URL('/proto.djtool.Djtool/Connect', this.hostname_).toString(),
      request,
      metadata || {},
      this.methodInfoConnect);
  }

  methodInfoDisconnect = new grpcWeb.AbstractClientBase.MethodInfo(
    Empty,
    (request: DisconnectRequest) => {
      return request.serializeBinary();
    },
    Empty.deserializeBinary
  );

  disconnect(
    request: DisconnectRequest,
    metadata: grpcWeb.Metadata | null): Promise<Empty>;

  disconnect(
    request: DisconnectRequest,
    metadata: grpcWeb.Metadata | null,
    callback: (err: grpcWeb.Error,
               response: Empty) => void): grpcWeb.ClientReadableStream<Empty>;

  disconnect(
    request: DisconnectRequest,
    metadata: grpcWeb.Metadata | null,
    callback?: (err: grpcWeb.Error,
               response: Empty) => void) {
    if (callback !== undefined) {
      return this.client_.rpcCall(
        new URL('/proto.djtool.Djtool/Disconnect', this.hostname_).toString(),
        request,
        metadata || {},
        this.methodInfoDisconnect,
        callback);
    }
    return this.client_.unaryCall(
    this.hostname_ +
      '/proto.djtool.Djtool/Disconnect',
    request,
    metadata || {},
    this.methodInfoDisconnect);
  }

}

