// Client code goes here...

// Note: Don't worry about the error sin your editor.
// Solana playground exposes the global "pg"
// and executes your code inside of an async function.

console.log(pg.PROGRAM_ID.toString());

const txHash = await pg.program.methods.hello().rpc();

console.log(txHash);