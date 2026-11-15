import { useEffect, useState } from "react";
import { api, type JobEvent } from "./api";

export type JobStatus = "running" | "done" | "error" | "cancelled";

export interface Job {
  id: string;
  type: "download" | "convert" | "edit" | "audio" | "record" | "photo";
  title: string;
  status: JobStatus;
  percent: number;
  speed?: string;
  eta?: string;
  elapsed?: number;
  output?: string;
  error?: string;
  lastLog?: string;
  /** Tamaño en bytes del archivo de entrada, para mostrar "antes → después" */
  inputSize?: number;
}

function apply(job: Job, e: JobEvent): Job {
  switch (e.kind) {
    case "progress":
      return { ...job, percent: e.percent ?? job.percent, speed: e.speed, eta: e.eta, elapsed: e.elapsed ?? job.elapsed };
    case "title":
      return { ...job, title: e.message ?? job.title, percent: 0 };
    case "log":
      return { ...job, lastLog: e.message };
    case "done":
      return { ...job, status: "done", percent: 100, output: e.output ?? job.output };
    case "error":
      return { ...job, status: "error", error: e.message };
    case "cancelled":
      return { ...job, status: "cancelled" };
  }
}

export function useJobs() {
  const [jobs, setJobs] = useState<Job[]>([]);

  useEffect(() => {
    const un = api.onJob((e) => setJobs((list) => list.map((j) => (j.id === e.id ? apply(j, e) : j))));
    return () => {
      un.then((f) => f());
    };
  }, []);

  const add = (job: Omit<Job, "status" | "percent">) =>
    setJobs((l) => [{ ...job, status: "running", percent: 0 }, ...l]);

  const fail = (id: string, error: string) =>
    setJobs((l) => l.map((j) => (j.id === id ? { ...j, status: "error", error } : j)));

  const clearFinished = () => setJobs((l) => l.filter((j) => j.status === "running"));

  /** Agrega el trabajo y ejecuta la llamada; si falla al arrancar, lo marca con error. */
  const run = async (job: Omit<Job, "status" | "percent">, start: () => Promise<void>) => {
    add(job);
    try {
      await start();
    } catch (e) {
      fail(job.id, String(e));
    }
  };

  return { jobs, add, fail, run, clearFinished };
}

export type JobsApi = ReturnType<typeof useJobs>;
