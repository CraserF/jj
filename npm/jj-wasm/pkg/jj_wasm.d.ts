/* tslint:disable */
/* eslint-disable */

export class JjSession {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    static clone(url: string, options: any): Promise<JjSession>;
    describe(options: any): Promise<any>;
    fetch(options: any): Promise<any>;
    static init(options: any): Promise<JjSession>;
    listRefs(ref_prefix?: string | null): Promise<any>;
    log(options: any): Promise<any>;
    new(options: any): Promise<any>;
    opLog(): Promise<any>;
    static open(options: any): Promise<JjSession>;
    push(options: any): Promise<any>;
    readRawObject(oid: string): Promise<any>;
    rebase(_options: any): Promise<any>;
    resolveRef(ref_name: string): Promise<any>;
    restore(options: any): Promise<any>;
    snapshot(options: any): Promise<any>;
    status(): Promise<any>;
    undo(): Promise<any>;
    writeRawObject(object_type: string, wrapped: Uint8Array): Promise<string>;
    writeRef(ref_name: string, oid: string): Promise<any>;
    readonly dir: string;
    readonly gitdir: string | undefined;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_jjsession_free: (a: number, b: number) => void;
    readonly jjsession_init: (a: any) => any;
    readonly jjsession_open: (a: any) => any;
    readonly jjsession_clone: (a: number, b: number, c: any) => any;
    readonly jjsession_fetch: (a: number, b: any) => any;
    readonly jjsession_push: (a: number, b: any) => any;
    readonly jjsession_log: (a: number, b: any) => any;
    readonly jjsession_status: (a: number) => any;
    readonly jjsession_snapshot: (a: number, b: any) => any;
    readonly jjsession_describe: (a: number, b: any) => any;
    readonly jjsession_new: (a: number, b: any) => any;
    readonly jjsession_rebase: (a: number, b: any) => any;
    readonly jjsession_restore: (a: number, b: any) => any;
    readonly jjsession_opLog: (a: number) => any;
    readonly jjsession_undo: (a: number) => any;
    readonly jjsession_listRefs: (a: number, b: number, c: number) => any;
    readonly jjsession_resolveRef: (a: number, b: number, c: number) => any;
    readonly jjsession_writeRef: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly jjsession_readRawObject: (a: number, b: number, c: number) => any;
    readonly jjsession_writeRawObject: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly jjsession_dir: (a: number) => [number, number];
    readonly jjsession_gitdir: (a: number) => [number, number];
    readonly wasm_bindgen__closure__destroy__h91d888a8b6f09f74: (a: number, b: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__hffb30fb49de9c351: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h96a0ce6d567e0458: (a: number, b: number, c: any, d: any) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
