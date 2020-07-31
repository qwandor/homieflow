/* eslint-disable @typescript-eslint/explicit-function-return-type */
import fetch from 'node-fetch';
import { username, password } from './globals';

describe('testing watermixer retreiving data', () => {
  it('attempting to get watermixer data with no credentials', async () => {
    const res = await fetch('http://localhost:8000/watermixer/getData', {
      method: 'GET',
    });
    expect(res.status).toEqual(401);
  });
  it('attempting to get watermixer data with invalid credentials', async () => {
    const res = await fetch('http://localhost:8000/watermixer/getData', {
      method: 'GET',
      headers: {
        username: 'randomUsername',
        password: 'randomPassword',
      },
    });
    expect(res.status).toEqual(401);
  });
  it('attempting to get watermixer data with valid credentials', async () => {
    const res = await fetch('http://localhost:8000/watermixer/getData', {
      method: 'GET',
      headers: {
        username: username,
        password: password,
      },
    });
    const resJson = await JSON.parse(await res.json());
    console.dir(await resJson);

    const entries = Object.entries(resJson);
    const realTypeLength = 2; // sorry for hardcoded, but i couldn't get length of interface
    expect(entries.length).toEqual(realTypeLength);

    expect(res.status).toEqual(200);
  });
});