import { FormEvent, ReactNode, useState } from "react";
import { LockKeyhole, ShieldCheck } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import { Button } from "../../shared/ui/button";

export function AuthGate({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient();
  const auth = useQuery({
    queryKey: ["auth-status"],
    queryFn: coreApi.authStatus,
    retry: false,
  });

  const [password, setPassword] = useState("");
  const mutation = useMutation({
    mutationFn: () =>
      auth.data?.setup_required
        ? coreApi.authSetup({ password })
        : coreApi.authLogin({ password }),
    onSuccess: async () => {
      setPassword("");
      await queryClient.invalidateQueries({ queryKey: ["auth-status"] });
      await queryClient.invalidateQueries({ queryKey: queryKeys.inventory });
    },
  });

  if (auth.isLoading) {
    return <AuthFrame icon={<ShieldCheck size={20} />} title="Loading" />;
  }

  if (
    !auth.data?.auth_required ||
    (auth.data.auth_required && auth.data.authenticated)
  ) {
    return <>{children}</>;
  }

  const setup = auth.data.setup_required;

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    mutation.mutate();
  }

  return (
    <AuthFrame
      icon={setup ? <ShieldCheck size={20} /> : <LockKeyhole size={20} />}
      title={setup ? "Set Local Password" : "Unlock Uprava"}
    >
      <form onSubmit={submit} className="mt-5 space-y-4">
        <label className="block text-sm font-medium text-[#27362f]">
          Password
          <input
            className="mt-2 h-10 w-full rounded-md border border-[#bfc8bc] bg-white px-3 text-base outline-none focus:border-[#2f7d6d]"
            minLength={12}
            onChange={(event) => setPassword(event.target.value)}
            type="password"
            value={password}
          />
        </label>
        {mutation.isError ? (
          <p className="text-sm text-[#a83f3a]">
            {mutation.error instanceof Error
              ? mutation.error.message
              : "Authentication failed"}
          </p>
        ) : null}
        <Button
          className="w-full"
          disabled={password.length < 12 || mutation.isPending}
          type="submit"
          variant="primary"
        >
          {setup ? "Save Password" : "Unlock"}
        </Button>
      </form>
    </AuthFrame>
  );
}

function AuthFrame({
  children,
  icon,
  title,
}: {
  children?: ReactNode;
  icon: ReactNode;
  title: string;
}) {
  return (
    <main className="flex min-h-screen items-center justify-center bg-[#f7f8f4] p-5 text-[#17211c]">
      <section className="w-full max-w-sm">
        <div className="mb-4 flex items-center gap-2 text-lg font-semibold">
          {icon}
          <h1>{title}</h1>
        </div>
        {children}
      </section>
    </main>
  );
}
