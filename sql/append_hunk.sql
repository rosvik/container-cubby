UPDATE hunks SET data = data || ?2, last_byte = last_byte + ?3 WHERE id = ?1;
