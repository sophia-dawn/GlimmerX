import { describe, it, expect, vi, beforeEach } from "vitest";
import * as api from "@/utils/api";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";

const mockedInvoke = vi.mocked(invoke);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("api invokeCommand", () => {
  it("calls tauri invoke with command and args", async () => {
    mockedInvoke.mockResolvedValueOnce({ ok: true });
    const result = await api.invokeCommand("test_cmd", { foo: "bar" });
    expect(mockedInvoke).toHaveBeenCalledWith("test_cmd", { foo: "bar" });
    expect(result).toEqual({ ok: true });
  });

  it("calls invoke without args", async () => {
    mockedInvoke.mockResolvedValueOnce(42);
    const result = await api.invokeCommand("no_args");
    expect(mockedInvoke).toHaveBeenCalledWith("no_args", undefined);
    expect(result).toBe(42);
  });
});

describe("api database commands", () => {
  it("dbCreate calls invoke", async () => {
    mockedInvoke.mockResolvedValueOnce({ path: "/test.db" });
    const result = await api.dbCreate("/test.db", "secret");
    expect(mockedInvoke).toHaveBeenCalledWith("db_create", {
      path: "/test.db",
      password: "secret",
    });
    expect(result.path).toBe("/test.db");
  });

  it("dbUnlock calls invoke", async () => {
    mockedInvoke.mockResolvedValueOnce({ path: "/test.db" });
    await api.dbUnlock("/test.db", "secret");
    expect(mockedInvoke).toHaveBeenCalledWith("db_unlock", {
      path: "/test.db",
      password: "secret",
    });
  });

  it("dbChangePassword calls invoke", async () => {
    mockedInvoke.mockResolvedValueOnce(undefined);
    await api.dbChangePassword("old", "new");
    expect(mockedInvoke).toHaveBeenCalledWith("db_change_password", {
      oldPassword: "old",
      newPassword: "new",
    });
  });

  it("dbCheckExists calls invoke", async () => {
    mockedInvoke.mockResolvedValueOnce(true);
    const result = await api.dbCheckExists("/test.db");
    expect(mockedInvoke).toHaveBeenCalledWith("db_check_exists", {
      path: "/test.db",
    });
    expect(result).toBe(true);
  });

  it("dbCheckAnyExists calls invoke", async () => {
    mockedInvoke.mockResolvedValueOnce(false);
    const result = await api.dbCheckAnyExists();
    expect(mockedInvoke).toHaveBeenCalledWith("db_check_any_exists", undefined);
    expect(result).toBe(false);
  });

  it("dbListRecent calls invoke", async () => {
    const recent = [
      { path: "/a.db", label: "a", lastOpened: "2026-01-01", exists: true },
    ];
    mockedInvoke.mockResolvedValueOnce(recent);
    const result = await api.dbListRecent();
    expect(mockedInvoke).toHaveBeenCalledWith("db_list_recent", undefined);
    expect(result).toEqual(recent);
  });

  it("dbRemoveRecent calls invoke", async () => {
    mockedInvoke.mockResolvedValueOnce(undefined);
    await api.dbRemoveRecent("/test.db");
    expect(mockedInvoke).toHaveBeenCalledWith("db_remove_recent", {
      path: "/test.db",
    });
  });

  it("dbLock calls invoke", async () => {
    mockedInvoke.mockResolvedValueOnce(undefined);
    await api.dbLock();
    expect(mockedInvoke).toHaveBeenCalledWith("db_lock", undefined);
  });
});

describe("api account commands", () => {
  it("accountCreate calls invoke", async () => {
    const input = { name: "Test", currency: "CNY" };
    mockedInvoke.mockResolvedValueOnce({ id: "1", ...input });
    const result = await api.accountCreate(input);
    expect(mockedInvoke).toHaveBeenCalledWith("account_create", { input });
    expect(result.id).toBe("1");
  });

  it("accountList calls invoke", async () => {
    mockedInvoke.mockResolvedValueOnce([]);
    await api.accountList();
    expect(mockedInvoke).toHaveBeenCalledWith("account_list", undefined);
  });

  it("accountUpdate calls invoke", async () => {
    mockedInvoke.mockResolvedValueOnce({ id: "1" });
    await api.accountUpdate("1", { name: "Updated" });
    expect(mockedInvoke).toHaveBeenCalledWith("account_update", {
      id: "1",
      input: { name: "Updated" },
    });
  });

  it("accountDelete calls invoke", async () => {
    mockedInvoke.mockResolvedValueOnce(undefined);
    await api.accountDelete("1");
    expect(mockedInvoke).toHaveBeenCalledWith("account_delete", { id: "1" });
  });

  it("accountBalance calls invoke", async () => {
    mockedInvoke.mockResolvedValueOnce(5000);
    const result = await api.accountBalance("1");
    expect(mockedInvoke).toHaveBeenCalledWith("account_balance", { id: "1" });
    expect(result).toBe(5000);
  });

  it("accountTransfer calls invoke", async () => {
    mockedInvoke.mockResolvedValueOnce("transfer-id");
    const result = await api.accountTransfer("1", "2", 100, "desc");
    expect(mockedInvoke).toHaveBeenCalledWith("account_transfer", {
      fromId: "1",
      toId: "2",
      amount: 100,
      description: "desc",
    });
    expect(result).toBe("transfer-id");
  });

  it("accountBatchCreate calls invoke", async () => {
    mockedInvoke.mockResolvedValueOnce([]);
    await api.accountBatchCreate([{ name: "A", currency: "CNY" }]);
    expect(mockedInvoke).toHaveBeenCalledWith("account_batch_create", {
      inputs: [{ name: "A", currency: "CNY" }],
    });
  });

  it("accountTransactions calls invoke with date range", async () => {
    mockedInvoke.mockResolvedValueOnce([]);
    await api.accountTransactions("1", "2026-01-01", "2026-04-01");
    expect(mockedInvoke).toHaveBeenCalledWith("account_transactions", {
      id: "1",
      fromDate: "2026-01-01",
      toDate: "2026-04-01",
    });
  });

  it("accountTransactions calls invoke without date range", async () => {
    mockedInvoke.mockResolvedValueOnce([]);
    await api.accountTransactions("1");
    expect(mockedInvoke).toHaveBeenCalledWith("account_transactions", {
      id: "1",
      fromDate: null,
      toDate: null,
    });
  });
});
