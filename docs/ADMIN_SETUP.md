# Admin Setup

## Bootstrap the first admin

New accounts are created with `admin: false`. To grant admin access to the first account:

1. Stop the server
2. Edit `prep-appointments/data/accounts.json`
3. Add `"admin": true` to the account you want as admin

Example:
```json
{
  "vor": {
    "account_name": "vor",
    "server_number": 140,
    "admin": true,
    ...
  }
}
```

4. Restart the server
5. Log in with that account — you'll see "Admin Resources" at the top of the sidebar
6. Use **Manage Admins** to grant or revoke admin for other accounts

## Admin capabilities

- **Admin Resources** section in the sidebar (visible only to admins)
- **Manage Admins** — list all accounts and grant/revoke admin privileges
