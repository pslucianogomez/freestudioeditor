import { useEffect, useRef } from "react";

/** Archivos soltados sobre la ventana, dirigidos a una herramienta. `seq` cambia en cada soltada. */
export interface Incoming {
  paths: string[];
  seq: number;
}

/** Llama a `cb` una vez por cada soltada nueva. */
export function useIncoming(incoming: Incoming | null, cb: (paths: string[]) => void) {
  const last = useRef(0);
  const cbRef = useRef(cb);
  cbRef.current = cb;
  useEffect(() => {
    if (incoming && incoming.seq !== last.current) {
      last.current = incoming.seq;
      cbRef.current(incoming.paths);
    }
  }, [incoming]);
}
