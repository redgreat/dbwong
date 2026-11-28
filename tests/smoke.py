"""HTTP acceptance check. Run against a disposable management database."""
import http.cookiejar
import json
import os
import secrets
import urllib.error
import urllib.request

origin = os.environ["DBW_ORIGIN"]
suffix = secrets.token_hex(6)
password = secrets.token_hex(16)


def browser():
    return urllib.request.build_opener(
        urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar())
    )


def call(client, path, body=None, expected=200, request_origin=origin):
    payload = None if body is None else json.dumps(body).encode()
    req = urllib.request.Request(origin + path, payload, {
        "Content-Type": "application/json", "Origin": request_origin
    })
    try:
        response = client.open(req, timeout=20)
    except urllib.error.HTTPError as err:
        response = err
    with response:
        assert response.status == expected, (path, response.status, expected)
        if response.headers.get_content_type() == "application/json":
            return json.load(response)


admin, dba = browser(), browser()
call(dba, "/api/connections", expected=401)
call(admin, "/api/login", {"name": "admin", "password": os.environ["DBW_ADMIN_PASSWORD"]})
account = {"name": suffix, "password": password}
call(admin, "/api/admin", {"action": "account", **account}, 403, "https://other.invalid")
call(admin, "/api/admin", {"action": "account", **account}, 204)
call(dba, "/api/login", account)
call(dba, "/api/admin", expected=403)
assert call(dba, "/api/connections") == []
call(admin, "/api/admin", {"action": "connection", "name": suffix, "dsn": password,
                           "environment": "test", "engine": "postgres"}, 204)
overview = call(admin, "/api/admin")
assert password not in json.dumps(overview)
def named_id(rows):
    return next(row["id"] for row in rows if row["name"] == suffix)

grant = {"action": "grant", "account": named_id(overview["accounts"]),
         "connection": named_id(overview["connections"])}
call(admin, "/api/admin", {**grant, "allow": True}, 204)
rows = call(dba, "/api/connections")
assert rows == [{"id": grant["connection"], "engine": "postgres",
                 "name": suffix, "environment": "test"}]
call(admin, "/api/admin", {**grant, "allow": False}, 204)
assert call(dba, "/api/connections") == []
connection_id = grant["connection"]
edit = {"action": "edit_connection", "id": connection_id, "name": suffix+"-edited",
        "environment": "production", "enabled": True}
call(dba, "/api/admin", edit, 403)
call(dba, "/api/admin/probe", {"connection": connection_id}, 403)
call(admin, "/api/admin", {**grant, "allow": True}, 204)
call(admin, "/api/admin", {**edit, "enabled": False}, 204)
assert call(dba, "/api/connections") == []
call(admin, "/api/admin/probe", {"connection": connection_id}, 409)
call(admin, "/api/admin", edit, 204)
assert call(dba, "/api/connections")[0]["name"] == edit["name"]
assert call(admin, "/api/admin/probe", {"connection": connection_id})["outcome"] == "probe.failed"
if probe_dsn := os.environ.get("DBW_TEST_PG_DSN"):
    call(admin, "/api/admin", {**edit, "dsn": probe_dsn}, 204)
    call(admin, "/api/admin", {**edit, "dsn": ""}, 204)
    assert call(admin, "/api/admin/probe", {"connection": connection_id})["outcome"] == "probe.ok"
    assert probe_dsn not in json.dumps(call(admin, "/api/admin"))
target = grant["account"]
status = {"action": "account_status", "account": target}
call(dba, "/api/admin", {**status, "enabled": False}, 403)
call(admin, "/api/admin", {**status, "enabled": False}, 204)
call(dba, "/api/connections", expected=401)
call(dba, "/api/login", account, 401)
call(admin, "/api/admin", {**status, "enabled": True}, 204)
call(dba, "/api/connections", expected=401)
call(dba, "/api/login", account)
new_password = secrets.token_hex(16)
call(admin, "/api/admin", {"action": "reset_password", "account": target,
                           "password": new_password}, 204)
call(dba, "/api/connections", expected=401)
call(dba, "/api/login", account, 401)
call(dba, "/api/login", {**account, "password": new_password})
call(dba, "/api/logout", {})
call(dba, "/api/connections", expected=401)
print("PASS: login, admin isolation, origin validation, grant, revoke, disable, restore, password reset, logout")



call(dba, "/api/admin/audit", expected=401)
call(dba, "/api/login", {**account, "password": new_password})
call(dba, "/api/admin/audit", expected=403)
call(admin, "/api/admin/audit?before=0", expected=400)
for _ in range(25):
    call(admin, "/api/admin", {"action": "session_hours", "hours": 8}, 204)
first_page = call(admin, "/api/admin/audit")
assert len(first_page["events"]) == 25
second_page = call(admin, f"/api/admin/audit?before={first_page['next_before']}")
assert all(event["id"] < first_page["next_before"] for event in second_page["events"])
assert any(event["action"] == "account.reset_password" for event in second_page["events"])
assert password not in json.dumps(second_page)
print("PASS: audit authorization, pagination, and credential exclusion")
