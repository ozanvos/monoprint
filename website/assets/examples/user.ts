type User = { name: string };

async function getUser(
  id: string,
): Promise<User> {
  const res = await fetch(
    `/api/users/${id}`,
  );

  return res.json();
}
